pub(crate) mod deserializers;
pub mod devel;
pub mod error;
pub mod game;
pub mod general;
pub mod mod_profile;
#[cfg(feature = "nexus")]
pub mod nexus;

use std::fmt::Debug;
// use fs_err as fs;
use std::fs;
use std::path::PathBuf;
use std::{env::var, str::FromStr};

use config::{Config, FileFormat};
use serde::{Deserialize, de::IntoDeserializer};
use tracing::{info, instrument, trace, warn};

use devel::DevelConf;
use error::{ConfigError, ConfigOpenError};
use game::{Game, GameAttr, GameTraitMut, Games};
use general::GeneralConf;
use mod_profile::ModProfile;
#[cfg(feature = "nexus")]
use nexus::NexusConf;

pub type Result<T> = miette::Result<T, ConfigError>;

#[derive(Debug, Deserialize, Clone)]
pub struct Configuration {
    pub general: GeneralConf,
    pub devel: Option<DevelConf>,
    #[cfg(feature = "nexus")]
    pub nexus: NexusConf,
}

#[instrument(level = "trace")]
pub fn get_main_config() -> Result<Configuration> {
    let mut cb = Config::builder().add_source(config::File::from_str(
        include_str!("config_defaults.toml"),
        FileFormat::Toml,
    ));

    trace!("checking: /etc/c77merge/config");
    cb = cb.add_source(config::File::with_name("/etc/c77merge/config").required(false));

    let config_path = match var("XDG_CONFIG_HOME") {
        Ok(path) => PathBuf::from(path),
        Err(_) => PathBuf::from(var("HOME").map_err(|e| ConfigError::MissingEnv {
            cause: e,
            var: "HOME",
        })?)
        .join(".config"),
    }
    .join("c77merge")
    .join("config.toml");
    trace!("checking: {}", config_path.display());
    let config_file: config::File<_, _> = config_path.into();
    cb = cb.add_source::<config::File<_, _>>(config_file.required(false));

    trace!("checking: ./config");
    cb = cb.add_source(config::File::with_name("./config").required(false));

    trace!("checking: C77_* environment variables");
    cb = cb.add_source(config::Environment::with_prefix("C77"));

    let config = cb.build()?;

    let mut configuration: Configuration = config.try_deserialize()?;

    if let Ok(testmode) = var("C77_TEST_MODE")
        && testmode.as_str() == "true"
    {
        if let Some(ref devel_conf) = configuration.devel {
            info!("Testing mode enabled! Overriding configuration to use the testing root");
            let root = devel_conf.testing_root.clone();
            configuration.general.modding_root = root.join("Games/Mods");
            configuration.general.steam_root = root.join(".steam");
        } else {
            warn!("Testing mode requested, but no devel configuration found; ignoring!");
        }
    } else {
        let modevar = var("C77_TEST_MODE");
        trace!("Testing mode not requested. Mode var: {modevar:?}");
    }

    Ok(configuration)
}

#[instrument(level = "trace", skip(config))]
pub fn get_games(config: &Configuration) -> Result<Vec<Game>> {
    let mut config_path: PathBuf = config.general.modding_root.clone();
    config_path.push("games.toml");

    let toml_str =
        fs::read_to_string(&config_path).map_err(|e| ConfigOpenError::new(config_path, e))?;

    let toml_data =
        toml_edit::Document::from_str(&toml_str).map_err(ConfigError::TomlSyntaxError)?;

    // the intermediate Games type is a hack to work around a serde limitation
    // see game.rs
    let mut games: Vec<Game> = Games::deserialize(toml_data.into_deserializer())
        .map_err(ConfigError::TomlSchemaError)?
        .into();

    for game in &mut games {
        game.expand_path(config);
    }

    Ok(games)
}

#[instrument(level = "trace", skip(config))]
pub fn get_mod_profile(
    config: &Configuration,
    game: &(impl GameAttr + Debug),
    profile_name: &'_ str,
) -> Result<ModProfile> {
    let mut config_path: PathBuf = config.general.modding_root.clone();
    config_path.push(game.path());
    config_path = shellexpand::path::tilde(&config_path)
        .canonicalize()
        .map_err(|e| ConfigOpenError::new(config_path, e))?;
    config_path.push(profile_name);
    config_path.set_extension("toml");

    let toml_str =
        fs::read_to_string(&config_path).map_err(|e| ConfigOpenError::new(config_path, e))?;

    let toml_data =
        toml_edit::Document::from_str(&toml_str).map_err(ConfigError::TomlSyntaxError)?;

    let profile: ModProfile = ModProfile::deserialize(toml_data.into_deserializer())
        .map_err(ConfigError::TomlSchemaError)?;

    Ok(profile)
}
