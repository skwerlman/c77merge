pub mod config_error;
pub mod game;
pub mod general;
pub mod mod_profile;
pub mod nexus;

use std::env::var;
use std::fs;
use std::path::PathBuf;

use config::{Config, FileFormat};
use tracing::{instrument, trace};

use config_error::ConfigError;
use general::GeneralConf;
use nexus::NexusConf;

use game::{Game, Games};
use mod_profile::ModProfile;
use serde::Deserialize;

pub type Result<T> = std::result::Result<T, ConfigError>;

#[derive(Deserialize, Debug, Clone)]
pub struct Configuration {
    pub general: GeneralConf,
    pub nexus: NexusConf,
}

impl Configuration {
    pub fn mask_api_key(&self) -> Self {
        let mut nexus = self.nexus.clone();
        nexus.api_key = "MASKED".to_string();

        Self {
            general: self.general.clone(),
            nexus,
        }
    }
}

#[instrument(level = "trace")]
pub fn get_main_config() -> Result<Configuration> {
    let config_base = match var("XDG_CONFIG_HOME") {
        Ok(path) => PathBuf::from(path),
        Err(_) => PathBuf::from(var("HOME")?).join(PathBuf::from(".config")),
    };

    let mut config_path = config_base
        .join(PathBuf::from("c77merge"))
        .to_str()
        .ok_or("failed to convert path to str")?
        .to_owned();

    config_path.push_str("/config");

    let mut cb = Config::builder().add_source(config::File::from_str(
        include_str!("config_defaults.toml"),
        FileFormat::Toml,
    ));

    trace!("checking: /etc/c77merge/config");
    cb = cb.add_source(config::File::with_name("/etc/c77merge/config").required(false));

    trace!("checking: {}", config_path);
    cb = cb.add_source(config::File::with_name(config_path.as_str()).required(false));

    trace!("checking: ./config");
    cb = cb.add_source(config::File::with_name("./config").required(false));

    trace!("checking: C77_* environment variables");
    cb = cb.add_source(config::Environment::with_prefix("C77"));

    let config = cb.build()?;

    let configuration = config.try_deserialize()?;

    Ok(configuration)
}

#[instrument(level = "trace", skip(config))]
pub fn get_games(config: &Configuration) -> Result<Vec<Game>> {
    let mut config_path: PathBuf = config.general.modding_root.clone();
    config_path.push("games.toml");

    // the intermediate Games type is a hack to work around a serde limitation
    // see game.rs
    let mut games: Vec<Game> =
        toml::from_str::<Games>(fs::read_to_string(config_path)?.as_str())?.into();

    for game in &mut games {
        game.expand_path(config);
    }

    Ok(games)
}

#[instrument(level = "trace", skip(config))]
pub fn get_mod_profile(
    config: &Configuration,
    game: &Game,
    profile_name: &'_ str,
) -> Result<ModProfile> {
    let mut config_path: PathBuf = config.general.modding_root.clone();
    config_path.push(&game.path);
    config_path = shellexpand::path::tilde(&config_path).canonicalize()?;
    config_path.push(profile_name);
    config_path.set_extension("toml");

    let profile = toml::from_str(fs::read_to_string(config_path)?.as_str())?;

    Ok(profile)
}
