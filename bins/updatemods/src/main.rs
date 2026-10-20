use std::cmp::Reverse;
use std::path::PathBuf;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use futures::StreamExt;
use miette::{Context, IntoDiagnostic, Report, Result, miette};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::task::JoinSet;
use tokio::time::sleep;
use tracing::{Instrument, Span, debug, error, field, info, trace_span, warn};
use tracing_subscriber::{EnvFilter, prelude::*};

use c77conf::game::prelude::*;
use c77conf::mod_profile::ModInfo;
use c77conf::{get_games, get_main_config, get_mod_profile};
use nexusapi::file_listing::File;
use nexusapi::{ApiClient, login};

#[derive(Parser, Debug)]
struct Cli {
    game: String,
    profile: String,
}

enum JoinSetReturnType {
    File {
        state: TaskState,
    },
    FileUpdate {
        state: TaskState,
    },
    DownloadLink {
        link: String,
        state: TaskState,
    },
    Unpack {
        file_path: PathBuf,
        state: TaskState,
    },
    Done,
}

#[derive(Debug, Clone)]
struct TaskState {
    span: Span,
    original_modinfo: ModInfo,
    original_file: Option<File>,
    new_modinfo: Option<ModInfo>,
    new_file: Option<File>,
}

impl TaskState {
    fn new(modinfo: ModInfo) -> Self {
        let span = trace_span!("update", mod_file = field::Empty);
        Self {
            span,
            original_modinfo: modinfo,
            original_file: None,
            new_modinfo: None,
            new_file: None,
        }
    }

    fn original_modinfo(&self) -> &ModInfo {
        &self.original_modinfo
    }

    fn original_file(&self) -> &File {
        if let Some(ref v) = self.original_file {
            v
        } else {
            panic!("Tried to access original_file before it was defined!")
        }
    }
    fn new_modinfo(&self) -> &ModInfo {
        if let Some(ref v) = self.new_modinfo {
            v
        } else {
            panic!("Tried to access new_modinfo before it was defined!")
        }
    }
    fn new_file(&self) -> &File {
        if let Some(ref v) = self.new_file {
            v
        } else {
            panic!("Tried to access new_file before it was defined!")
        }
    }
    fn set_original_file(&mut self, original_file: File) {
        self.original_file = Some(original_file);
    }
    fn set_new_modinfo(&mut self, new_modinfo: ModInfo) {
        self.new_modinfo = Some(new_modinfo);
    }
    fn set_new_file(&mut self, new_file: File) {
        self.new_file = Some(new_file);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    libc77::init!();

    let main_span = trace_span!("main").entered();

    let cli = trace_span!("cli").in_scope(Cli::parse);

    let config_span = trace_span!("configs").entered();
    let settings = Arc::new(get_main_config()?);
    let nexus_conf = Arc::new(settings.nexus.clone());

    let games = get_games(&settings)?;

    let game: GameDomain = games
        .into_iter()
        .find(|g| g.aliases().contains(&cli.game))
        .ok_or_else(|| miette!("Game not found"))?
        .try_into()?;

    let profile = get_mod_profile(&settings, &game, &cli.profile)?;
    drop(config_span.exit());

    let login_span = trace_span!("login").entered();
    // start login process before hashing mods
    let api_future = tokio::task::spawn(login(nexus_conf));

    // await login process since we need the api client to get mod info
    let (api_client, _userinfo, limiter) = match api_future.await.into_diagnostic()? {
        Ok((partial_api_client, userinfo, limiter)) => {
            let full_api_client = partial_api_client.with_domain(game.domain());
            (full_api_client, userinfo, limiter)
        }
        Err(err) => {
            error!("{}", Report::new(err));
            sleep(Duration::from_millis(75)).await;
            exit(1)
        }
    };
    login_span.exit();

    let api_client_clone = api_client.clone();

    // schedule periodic rate limiter refreshes
    tokio::spawn(async move {
        loop {
            let resp = api_client_clone.clone().refresh_limits(&limiter).await;
            if let Err(err) = resp {
                error!("{}", Report::new(err));
            }
            tokio::time::sleep(Duration::from_secs(180)).await;
        }
    });

    let mut pending_tasks = JoinSet::new();

    // get information about each mod from the nexus
    for mod_ in profile.mods.into_iter() {
        enqueue_get_file_info(&mut pending_tasks, api_client.clone(), TaskState::new(mod_));
    }

    while let Some(res) = pending_tasks.join_next().await {
        match res.map_err(|e| miette!("Error joining task: {e}"))?? {
            JoinSetReturnType::File { state } => {
                // use the info we got from the nexus to ask about updates to the mod
                enqueue_get_update(&mut pending_tasks, api_client.clone(), state);
            }
            JoinSetReturnType::FileUpdate { state } => {
                // get info about new versions of files
                enqueue_get_download_link(&mut pending_tasks, api_client.clone(), state);
            }
            JoinSetReturnType::DownloadLink { link, state } => {
                enqueue_download_file(
                    &mut pending_tasks,
                    api_client.clone(),
                    game.download_dir(),
                    link,
                    state,
                );
            }
            JoinSetReturnType::Unpack { file_path, state } => {
                enqueue_unpack_file(&mut pending_tasks, file_path, state);
            }
            JoinSetReturnType::Done => {}
        }
    }

    drop(main_span.exit());
    Ok(())
}

fn enqueue_get_file_info(
    queue: &mut JoinSet<Result<JoinSetReturnType>>,
    client: ApiClient,
    mut state: TaskState,
) {
    let span = state.span.clone();
    queue.spawn(
        async move {
            let span = trace_span!(parent: &state.span, "collectfileinfo");
            let file = nexusapi::get_file_by_id(
                client,
                state.original_modinfo().id,
                state.original_modinfo().fid,
            )
            .instrument(span.clone())
            .await?;
            let span = span.enter();

            state.span.record("mod_file", file.mod_name.clone());

            state.set_original_file(file);

            drop(span);

            Ok(JoinSetReturnType::File { state })
        }
        .instrument(span),
    );
}

fn enqueue_get_update(
    queue: &mut JoinSet<Result<JoinSetReturnType>>,
    client: ApiClient,
    mut state: TaskState,
) {
    let span = state.span.clone();
    queue.spawn(
        async move {
            let span = trace_span!(parent: &state.span, "getupdate");
            let file_listing =
                nexusapi::get_file_listing(client.clone(), state.original_modinfo().id)
                    .instrument(span.clone())
                    .await?;

            let span = span.enter();

            let mut changes = file_listing.changelog().to_owned();

            // the api _should_ return these in the right order but i dont trust it :)
            changes.sort_by_key(|c| Reverse(c.new_file_id));

            // pop changes until we've removed the one where our file was uploaded
            while let Some(e) = changes.pop() {
                // trace!("popped {e}");
                if e.new_file_id == state.original_file().file_id {
                    break;
                }
            }

            let update = changes.collapse(state.original_file().file_id);

            drop(span);

            if update.is_change() {
                info!(
                    "Pending changes for {}: {update}",
                    state.original_file().mod_name
                );
                // state.set_new_modinfo(
                //     state
                //         .original_modinfo()
                //         .clone_to_new_with_fid(update.new_file_id),
                // );
                let span = trace_span!(parent: &state.span, "newfileinfo");
                let new_file = nexusapi::get_file_by_id(
                    client,
                    state.original_modinfo().id,
                    update.new_file_id,
                )
                .instrument(span)
                .await?;
                state.set_new_file(new_file);
                Ok(JoinSetReturnType::FileUpdate { state })
            } else {
                Ok(JoinSetReturnType::Done)
            }
        }
        .instrument(span),
    );
}

fn enqueue_get_download_link(
    queue: &mut JoinSet<Result<JoinSetReturnType>>,
    client: ApiClient,
    state: TaskState,
) {
    let span = state.span.clone();
    queue.spawn(
        async move {
            let span = trace_span!(parent: &state.span, "getlinks");
            let links = nexusapi::get_download_link(
                client,
                state.original_modinfo().id,
                state.new_file().file_id,
            )
            .instrument(span.clone())
            .await?;
            let span = span.enter();
            let link = links
                .get_link_by_shortname("Nexus CDN")
                .ok_or(miette!("Failed to get download link for Nexus CDN!"))?
                .to_owned();
            drop(span);
            Ok(JoinSetReturnType::DownloadLink { link, state })
        }
        .instrument(span),
    );
}

// TODO move the actual download logic into nexusapi so it can use the rate limiter
fn enqueue_download_file(
    queue: &mut JoinSet<Result<JoinSetReturnType>>,
    client: ApiClient,
    download_dir: PathBuf,
    link: String,
    state: TaskState,
) {
    let span = state.span.clone();
    queue.spawn(async move {
        let mut file_path = download_dir;
        file_path.push(&state.new_file().file_name);

        if file_path.exists() {
            let hash = libc77::hash_mod::hash_mod(&file_path)?;

            // TODO use the SHA1 we get via the hack to check before resorting to an api call
            if let Ok(lookup) = nexusapi::get_by_hash(client, hash).await {
                let lfc = trace_span!(parent: &state.span, "localfilecheck").entered();
                if lookup.mod_details.mod_id != state.original_modinfo().id || lookup.file_details.file_id != state.new_file().file_id {

                    warn!(
                        "File already existed, but appears to belong to a different mod! We expected ({}, {}) but got ({}, {})! Overwriting {file_path:?}", state.original_modinfo().id, state.new_file().file_id, lookup.mod_details.mod_id, lookup.file_details.file_id
                    );
                } else {
                    info!("Already have file, skipping download: {file_path:?}");

                    return Ok(JoinSetReturnType::Unpack {
                        file_path,
                        state,
                    })
                }
                drop(lfc);
            } else {
                warn!(
                    "File already existed, but does not appear to be valid. Overwriting {file_path:?}"
                );
            }
        }


        // exit(0);

        let dfspan = trace_span!(parent: &state.span, "downloadfile");

        dfspan.in_scope(async || -> Result<()> {
        debug!("downloading file to {file_path:?}");

        let mut file = fs::File::create(&file_path)
            .await
            .into_diagnostic()
            .wrap_err("Failed to create file")?;

        let mut stream = reqwest::get(link)
            .await
            .into_diagnostic()
            .wrap_err("Failed to start download")?
            .bytes_stream();

        while let Some(chunk_res) = stream.next().await {
            let chunk = chunk_res
                .into_diagnostic()
                .wrap_err("Failed to fetch chunk data")?;
            file.write_all(&chunk)
                .await
                .into_diagnostic()
                .wrap_err("Failed to write chunk data")?;
        }

        file.flush().await.into_diagnostic()?;

        Ok(())
        }).await?;



        // TODO: validate download matches md5

        Ok(JoinSetReturnType::Unpack {
            file_path,
            state,
        })
    }
    .instrument(span),);
}

fn enqueue_unpack_file(
    queue: &mut JoinSet<Result<JoinSetReturnType>>,
    file_path: PathBuf,
    state: TaskState,
) {
    // let new_modinfo = ModInfo {
    //     id: state.original_modinfo().id,
    //     fid: state.new_file().file_id,
    //     archive: file_path.file_name(),
    //     unpacked,
    //     ..Default::default()
    // };
}
