use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use miette::{Result, miette};
use ovfs::{Id, ovfs::OVFSTree};
use ovfs::{mount, unshare};
use tracing::{debug, info, instrument, trace, trace_span};
use tracing_subscriber::fmt::{format, format::FmtSpan};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use acf::{AcfToken, AcfTokenStream, StructuredAcfTokenStream};
use c77conf::game::Game;
use c77conf::mod_profile::ModProfile;
use c77conf::{Configuration, get_games, get_main_config, get_mod_profile};

#[derive(Parser, Debug, Clone)]
struct Cli {
    game: String,
    profile: String,
    #[command(subcommand)]
    mode: OVFSMode,
}

#[derive(Debug, Subcommand, Clone)]
enum OVFSMode {
    Enable,
    Disable,
    Wrap {
        #[clap(raw = true, required = true, last = false)]
        command: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    human_panic::setup_panic!();
    let format = format::format().pretty();
    let filter = EnvFilter::try_from_env("C77_LOG").unwrap_or_else(|_| EnvFilter::new("debug"));

    fmt()
        .event_format(format)
        .with_env_filter(filter)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .finish()
        .init();

    let main_span = trace_span!("main").entered();

    let cli_span = trace_span!("cli").entered();
    let cli = Cli::parse();
    trace!("cli: {cli:?}");
    cli_span.exit();

    let config_span = trace_span!("configs").entered();
    let settings: Arc<Configuration> = Arc::new(get_main_config()?);
    trace!("config: {:?}", settings.mask_api_key());

    let games = get_games(&settings)?;
    trace!("games: {games:?}");

    let game = Arc::new(
        games
            .into_iter()
            .find(|g| g.aliases.contains(&cli.game))
            .ok_or_else(|| miette!("Game not found"))?,
    );
    trace!("game: {game:?}");
    game.ensure_dirs()?;

    let profile: Arc<ModProfile> = Arc::new(get_mod_profile(&settings, &game, &cli.profile)?);
    trace!("profile: {profile:?}");

    config_span.exit();

    match cli.mode {
        OVFSMode::Enable => {
            let merged_into = enable(&settings, &game, &profile);
            // info!("Modlist merged onto: {}", merged_into.display())
        }
        OVFSMode::Disable => {}
        OVFSMode::Wrap { command: cmd } => {
            // let merged_into = wrap(&settings, &game, &profile);
            // info!("Modlist merged onto: {}", merged_into.display())
        }
    }

    main_span.exit();
    Ok(())
}

#[instrument(level = "trace", skip(settings, game, profile))]
fn enable(settings: &Configuration, game: &Game, profile: &ModProfile) -> Result<()> {
    let acf_file = settings
        .general
        .steam_root
        .clone()
        .join("root/steamapps/")
        .join(format!("appmanifest_{}.acf", game.steamid));
    debug!("acf file for game: {acf_file:?}");

    let acf_handle = File::open(acf_file).map_err(|e| miette!("ACF Read error: {e}"))?;
    let mut stream = StructuredAcfTokenStream::new(AcfTokenStream::new(acf_handle));
    let path = vec!["AppState", "installdir"];
    stream
        .select_path(&path)
        .map_err(|e| miette!("ACF Parse error: {e}"))?;
    let installdir = match stream
        .expect_next()
        .map_err(|e| miette!("ACF Parse error: {e}"))
    {
        Ok(AcfToken::String(dir)) => Ok(dir),
        Ok(t) => Err(miette!("ACF Parse error: Unexpected token: {t:?}")),
        Err(e) => Err(e),
    }?;

    let game_dir = settings
        .general
        .steam_root
        .clone()
        .join("root/steamapps/common/")
        .join(installdir);

    debug!("game_dir: {game_dir:?}");

    let tree = build_tree(game, profile, game_dir)?;

    let me = unshare(Id::root(), true)?;

    mount(tree)?;

    let _fake_root = unshare(me, false)?;

    Ok(())
}

fn build_tree(game: &Game, profile: &ModProfile, mountpoint: PathBuf) -> Result<OVFSTree> {
    let overwrite = game.overwrite_dir();
    let mut tree = OVFSTree::new(mountpoint.clone(), game.work_dir(), overwrite);

    // mods are added in order; 1st in config is topmost
    for mod_ in &profile.mods {
        let path = game.mods_dir().join(&mod_.unpacked);
        tree.with_lower(path);
    }

    // set lowest dir to the directory we are gonna shadow
    tree.with_lower(mountpoint);

    Ok(tree)
}
