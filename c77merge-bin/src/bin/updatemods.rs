// use std::sync::Arc;
use std::error::Error;

// use c77merge::hash_mod::hash_all_in_dir;
// use c77merge::time::format_elapsed;
// use clap::Parser;
// use tracing::{debug, error, info, instrument, trace, trace_span};
// use tracing_subscriber::{EnvFilter, fmt, prelude::*};

// use c77conf::{Configuration, get_games, get_main_config, get_mod_profile};
// use nexusapi::login;

// #[derive(Parser, Debug)]
// struct Cli {
//     game: String,
//     profile: String,
// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    //     let timer_start = Instant::now();
    //     // let log_env = env_logger::Env::default()
    //     //     .filter("C77_LOG")
    //     //     .write_style("C77_LOG_STYLE");
    //     // env_logger::Builder::from_env(log_env)
    //     //     .format_timestamp_millis()
    //     //     .init();
    //     // debug!(
    //     //     "logging initialized ({})",
    //     //     format_elapsed(timer_start.elapsed())
    //     // );

    //     let timer_start = Instant::now();
    //     human_panic::setup_panic!();
    //     debug!(
    //         "panic handler initialized ({})",
    //         format_elapsed(timer_start.elapsed())
    //     );

    //     let timer_start = Instant::now();
    //     let cli = Cli::parse();
    //     debug!("cli parsed ({})", format_elapsed(timer_start.elapsed()));
    //     trace!("cli: {:?}", cli);

    //     let timer_start = Instant::now();
    //     let settings: Arc<Configuration> = Arc::new(get_main_config()?);
    //     debug!("config parsed ({})", format_elapsed(timer_start.elapsed()));
    //     trace!("config: {:?}", settings);

    //     let timer_start = Instant::now();
    //     let games = get_games(&settings)?;
    //     debug!("games parsed ({})", format_elapsed(timer_start.elapsed()));
    //     trace!("games: {:?}", games);

    //     let game = games
    //         .iter()
    //         .find(|g| g.aliases.contains(&cli.game))
    //         .ok_or("Game not found")?;

    //     info!("Looking at mods for {}", game.name);

    //     let timer_start = Instant::now();
    //     let profile = get_mod_profile(&settings, game, cli.profile)?;
    //     debug!("profile parsed ({})", format_elapsed(timer_start.elapsed()));
    //     trace!("profile: {:?}", profile);

    //     // start login process before hashing mods
    //     let api_future = tokio::task::spawn(login(settings));

    //     let mut game_downloads = game.path.clone();
    //     game_downloads.push("Download");

    //     let (mod_hashes, errors) = hash_all_in_dir(game_downloads);

    //     for err in errors {
    //         error!("failed to hash a file: {}", err)
    //     }

    //     // await login process since we need the api client to get mod info
    //     let (api_client, userinfo, limiter) = api_future.await??;

    //     // todo!("dying on purpose so we dont hit the api");

    //     // let mut mod_futures: Vec<_> = Vec::new();
    //     // let mut mods: Vec<_> = Vec::new();

    //     // let mut errors = Vec::new();

    //     // for hash in mod_hashes {
    //     //     mod_futures.push(nexusapi::get_by_hash(&api_client, hash));
    //     // }

    //     // for future in mod_futures {
    //     //     mods.push(future.await.map_err(|e| errors.push(e)));
    //     // }

    //     // let mod_futures: Vec<_> = mod_hashes
    //     //     .into_iter()
    //     //     .map(async |h| nexusapi::get_by_hash(&api_client, h).await)
    //     //     .collect::<Vec<_>>();

    //     // let mods: Vec<_> = join_all(mod_futures)
    //     //     .await
    //     //     .into_iter()
    //     //     .filter_map(|f| f.map_err(|e| errors.push(e)).ok())
    //     //     .collect();

    //     // for err in errors {
    //     //     error!("failed to get mod info: {}", err)
    //     // }

    //     // for _mod_info in mods {
    //     //     todo!()
    //     // }

    Ok(())
}
