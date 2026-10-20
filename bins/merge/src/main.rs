use std::fmt::Debug;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use clap::Parser;
use itertools::Itertools;
use miette::{IntoDiagnostic, Result, WrapErr, diagnostic, miette};
use tracing::{debug, info, instrument, trace, trace_span};
use tracing_subscriber::{EnvFilter, prelude::*};

use c77conf::game::prelude::*;
use c77conf::mod_profile::{Mapping, ModProfile};
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
    libc77::init!();

    let main_span = trace_span!("main").entered();

    let cli: Cli;

    {
        let cli_span = trace_span!("cli").entered();
        cli = Cli::parse();
        trace!("cli: {cli:?}");
        cli_span.exit();
    }

    let settings: Arc<Configuration>;
    let game: Arc<Game>;
    let profile: Arc<ModProfile>;
    let game_dir: PathBuf;

    {
        let config_span = trace_span!("configs").entered();
        settings = Arc::new(get_main_config().wrap_err("Failed to get the main configuration")?);
        trace!("config: {settings:?}");

        {
            let games = get_games(&settings).wrap_err("Failed to get the games configuration")?;
            trace!("games: {games:?}");

            game = Arc::new(
                games
                    .iter()
                    .find(|g| g.aliases().contains(&cli.game))
                    .ok_or_else(|| {
                        let mut help_txt = String::from("Known games: ");
                        for g in games.iter() {
                            help_txt += "\n";
                            help_txt += g.name();
                            if !g.aliases().is_empty() {
                                help_txt += "\n  Aliases: ";
                                let mut skip_sep = true;
                                for alias in g.aliases().iter() {
                                    if skip_sep {
                                        skip_sep = false;
                                    } else {
                                        help_txt += ", ";
                                    }
                                    help_txt += alias;
                                }
                            }
                        }
                        diagnostic!(help = help_txt, "Game not found: {}", cli.game)
                    })?
                    .clone(),
            );
        }
        trace!("game: {game:?}");
        game.ensure_dirs()?;

        profile = Arc::new(get_mod_profile(&settings, game.as_ref(), &cli.profile)?);
        trace!("profile: {profile:?}");

        game_dir = get_game_dir(&settings, &game)?;
        config_span.exit();
    }

    info!(
        "Applying profile '{}' for {}",
        &profile.profile.name,
        game.name()
    );

    {
        let vfs_span = trace_span!("vfs").entered();

        let mut trees = Vec::new();

        let early_maps: Vec<Mapping> = profile
            .mappings
            .iter()
            .filter(|m| m.early)
            .cloned()
            .collect();

        debug!("early maps: {early_maps:?}");

        let targets: Vec<PathBuf> = profile
            .mods
            .iter()
            .map(|m| m.child_dir.clone())
            .unique()
            .collect();

        debug!("mod targets: {targets:?}");

        let late_maps: Vec<Mapping> = profile
            .mappings
            .iter()
            .filter(|m| !m.early)
            .cloned()
            .collect();
        debug!("late maps: {late_maps:?}");

        let work_dir = game.work_dir();

        for map in early_maps.into_iter() {
            let tree = OVFSTree::new(
                map.mapped,
                work_dir.clone(),
                map.real,
                vec![work_dir.join("ignore")],
            );
            trees.push(tree);
        }

        for tgt in targets.iter() {
            debug!("building vfs tree for {tgt:?}");
            let mut tree: OVFSTree;

            {
                let mountpoint = game_dir.join(tgt);

                let lowers: Vec<PathBuf> = profile
                    .mods
                    .iter()
                    .filter_map(|mod_| {
                        if &mod_.child_dir == tgt {
                            mod_.unpacked.as_ref().map(|s| game.mods_dir().join(s))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                // lowers.push(game_dir.join(&tgt));

                tree = OVFSTree::new(
                    // mountpoint
                    mountpoint.clone(),
                    // workdir
                    game.work_dir(),
                    // overwrite
                    game.overwrite_dir().join(tgt),
                    // lowers
                    lowers,
                );

                // set lowest dir to the directory we are gonna shadow
                tree.with_lower(mountpoint);
            }
            trees.push(tree);
        }

        for map in late_maps.into_iter() {
            let tree = OVFSTree::new(
                map.mapped,
                work_dir.clone(),
                map.real,
                vec![work_dir.join("ignore")],
            );
            trees.push(tree);
        }

        enter_vfs(trees).wrap_err("Failed to create and enter the VFS")?;

        {
            let run_span = trace_span!("run_game").entered();
            run_game(&cli.command[0], &cli.command[1..])?;
            run_span.exit();
        }

        vfs_span.exit();
    }

    main_span.exit();
    Ok(())
}

#[instrument(level = "trace", skip(settings, game))]
fn get_game_dir(settings: &Configuration, game: &Game) -> Result<PathBuf> {
    let game_paths = acf::lookup::game_paths(&settings.general.steam_root)?;

    let acf_file = PathBuf::from(
        game_paths
            .get(game.steamid())
            .ok_or(miette!("Failed to convert game path to PathBuf"))?,
    );

    debug!("acf file for game: {acf_file:?}");

    let installdir: PathBuf;
    {
        let acf_span =
            trace_span!("read_acf", acf_file = acf_file.to_string_lossy().as_ref()).entered();
        installdir = acf::select_string_in_file(acf_file, &["AppState", "installdir"])?.into();
        acf_span.exit();
    }

    let game_dir = settings
        .general
        .steam_root
        .clone()
        .join("root/steamapps/common/")
        .join(installdir);

    debug!("game_dir: {game_dir:?}");

    Ok(game_dir)
}

// #[instrument(level = "trace", skip(game, profile))]
// fn build_tree<'a>(
//     game: &'a Game,
//     profile: &'a ModProfile,
//     game_dir: &'a Path,
//     target: &'a Path,
// ) -> OVFSTree<'a> {
// }

#[instrument(level = "trace", skip(trees))]
fn enter_vfs(trees: Vec<OVFSTree>) -> Result<()> {
    debug!("entering vfs");
    let me = unshare(Id::root(), true).wrap_err("Failed to unshare into fake-root context")?;

    // let last_idx = trees.len() - 1;

    for (index, mut tree) in trees.into_iter().enumerate() {
        tree.set_index(index);
        tree.ensure_dirs()?;
        mount(tree).wrap_err("Failed to mount VFS")?;
    }

    let _fake_root = unshare(me, false).wrap_err("Failed to drop fake-root context")?;

    Ok(())
}

fn run_game(command: &str, args: &[String]) -> Result<()> {
    info!("Launching game...");
    debug!("command: {command:?}");
    debug!("args: {args:?}");
    let mut child = Command::new(command)
        .args(args.iter().map(|arg| arg.as_str()))
        .spawn()
        .into_diagnostic()
        .wrap_err("Command error")?;

    let status = child.wait().into_diagnostic().wrap_err("Command error")?;
    info!("Game exited with status: {status}");

    Ok(())
}
