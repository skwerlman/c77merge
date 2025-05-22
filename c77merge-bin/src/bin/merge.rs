use std::env::args_os;
use std::fs::File;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use clap::Parser;
use itertools::Itertools;
use miette::{Result, miette};
use tracing::{debug, info, instrument, trace, trace_span};
use tracing_subscriber::fmt::{format, format::FmtSpan};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use acf::{AcfToken, AcfTokenStream, StructuredAcfTokenStream};
use c77conf::game::Game;
use c77conf::mod_profile::{ModInfo, ModProfile};
use c77conf::{Configuration, get_games, get_main_config, get_mod_profile};
use ovfs::{Id, ovfs::OVFSTree};
use ovfs::{mount, unshare};

#[derive(Parser, Debug, Clone)]
struct Cli {
    game: String,
    profile: String,
    #[clap(raw = true, required = true)]
    command: Vec<String>,
}

fn main() -> Result<()> {
    human_panic::setup_panic!();
    let format = format::format().compact();
    let filter = EnvFilter::try_from_env("C77_LOG").unwrap_or_else(|_| EnvFilter::new("debug"));

    fmt()
        .event_format(format)
        .with_env_filter(filter)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .finish()
        .init();

    c77merge::header!();

    let main_span = trace_span!("main").entered();

    let cli_span = trace_span!("cli").entered();
    trace!("raw args: {:?}", args_os());
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

    let game_dir = get_game_dir(&settings, &game)?;
    config_span.exit();

    let vfs_span = trace_span!("vfs").entered();

    let targets: Vec<PathBuf> = profile
        .mods
        .iter()
        .map(|m| m.child_dir.clone())
        .unique()
        .collect();

    debug!("targets: {targets:?}");

    let mut trees = Vec::new();
    for tgt in targets {
        debug!("building vfs tree for {tgt:?}");
        trees.push(build_tree(&game, &profile, game_dir.join(&tgt), |m| {
            m.child_dir == tgt
        })?);
    }

    enter_vfs(trees)?;

    let run_span = trace_span!("run_game").entered();
    run_game(&cli.command[0], &cli.command[1..])?;
    run_span.exit();

    vfs_span.exit();

    main_span.exit();
    Ok(())
}

#[instrument(level = "trace", skip(settings, game))]
fn get_game_dir(settings: &Configuration, game: &Game) -> Result<PathBuf> {
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

    Ok(game_dir)
}

#[instrument(level = "trace", skip(game, profile, filter))]
fn build_tree(
    game: &Game,
    profile: &ModProfile,
    mountpoint: PathBuf,
    filter: impl Fn(&ModInfo) -> bool,
) -> Result<OVFSTree> {
    let overwrite = game.overwrite_dir();
    let mut tree = OVFSTree::new(mountpoint.clone(), game.work_dir(), overwrite);

    // mods are added in order; 1st in config is topmost
    for mod_ in &profile.mods {
        if filter(mod_) {
            let path = game.mods_dir().join(&mod_.unpacked);
            tree.with_lower(path);
        }
    }

    // set lowest dir to the directory we are gonna shadow
    tree.with_lower(mountpoint);

    Ok(tree)
}

#[instrument(level = "trace", skip(trees))]
fn enter_vfs(trees: Vec<OVFSTree>) -> Result<()> {
    debug!("entering vfs");
    let me = unshare(Id::root(), true)?;

    let last_idx = trees.len() - 1;

    for (index, tree) in trees.into_iter().enumerate() {
        mount(tree, index != last_idx)?;
    }

    let _fake_root = unshare(me, false)?;

    Ok(())
}

fn run_game(command: &str, args: &[String]) -> Result<()> {
    info!("Launching game...");
    debug!("command: {command:?}");
    debug!("args: {args:?}");
    let mut child = Command::new(command)
        .args(args.iter().map(|arg| arg.as_str()))
        .spawn()
        .map_err(|e| miette!("Command error: {e}"))?;

    let status = child.wait().map_err(|e| miette!("Command error: {e}"))?;
    info!("Game exited with status: {status}");

    Ok(())
}
