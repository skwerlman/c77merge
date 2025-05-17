use std::fs::File;
use std::sync::Arc;

use clap::{Parser, ValueEnum};
use miette::{Result, miette};
use tracing::{debug, info, instrument, trace, trace_span};
use tracing_subscriber::fmt::{format, format::FmtSpan};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use acf::{AcfTokenStream, StructuredAcfTokenStream};
use c77conf::game::Game;
use c77conf::mod_profile::ModProfile;
use c77conf::{Configuration, get_games, get_main_config, get_mod_profile};

#[derive(Parser, Debug, Clone)]
struct Cli {
    mode: OVFSMode,
    game: String,
    profile: String,
}

#[derive(Debug, ValueEnum, Clone, Copy)]
enum OVFSMode {
    Enable,
    Disable,
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
    trace!("cli: {:?}", cli);
    cli_span.exit();

    let config_span = trace_span!("configs").entered();
    let settings: Arc<Configuration> = Arc::new(get_main_config()?);
    trace!("config: {:?}", settings);

    let games = get_games(&settings)?;
    trace!("games: {:?}", games);

    let game = Arc::new(
        games
            .into_iter()
            .find(|g| g.aliases.contains(&cli.game))
            .ok_or_else(|| miette!("Game not found"))?,
    );
    trace!("game: {:?}", game);

    let profile: Arc<ModProfile> = Arc::new(get_mod_profile(&settings, &game, &cli.profile)?);
    trace!("profile: {:?}", profile);

    config_span.exit();

    match cli.mode {
        OVFSMode::Enable => {
            let merged_into = enable(&settings, &game, &profile);
            // info!("Modlist merged onto: {}", merged_into.display())
        }
        OVFSMode::Disable => {}
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
    debug!("acf file for game: {:?}", acf_file);

    let acf_handle = File::open(acf_file).map_err(|e| miette!("ACF Read error: {}", e))?;
    let mut stream = StructuredAcfTokenStream::new(AcfTokenStream::new(acf_handle));
    let path = vec!["AppState", "installdir"];
    stream
        .select_path(&path)
        .map_err(|e| miette!("ACF Parse error: {}", e))?;
    let installdir = stream
        .expect_next()
        .map_err(|e| miette!("ACF Parse error: {}", e))?;
    debug!("installdir: {:?}", installdir);

    Ok(())
}
