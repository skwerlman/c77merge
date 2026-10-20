use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use tracing::{debug, instrument};

use crate::{
    Configuration, Result,
    error::{
        ConfigError::{self},
        DirectoryCheckError, DirectoryCreationError,
    },
};

#[derive(Deserialize, Debug, Clone)]
pub struct Game {
    name: String,
    path: PathBuf,
    #[serde(default)]
    aliases: Vec<String>,
    domain: Option<String>,
    steamid: u32,
}

#[derive(Debug, Clone)]
pub struct GameDomain {
    name: String,
    path: PathBuf,
    aliases: Vec<String>,
    domain: String,
    steamid: u32,
}

pub trait GameTrait: GameAttr {
    fn download_dir(&self) -> PathBuf {
        self.path().join("Download")
    }
    fn mods_dir(&self) -> PathBuf {
        self.path().join("Mods")
    }
    fn overwrite_dir(&self) -> PathBuf {
        self.path().join("Overwrite")
    }
    fn work_dir(&self) -> PathBuf {
        self.path().join(".ovfs")
    }
    fn profile(&self, name: &str) -> PathBuf {
        self.path().join(name).with_extension("toml")
    }

    #[instrument(level = "trace", skip(self), fields(game = %self.name()))]
    fn ensure_dirs(&self) -> Result<()> {
        let modfiles = self.path().to_owned();
        let download = self.download_dir();
        let mods = self.mods_dir();
        let overwrite = self.overwrite_dir();
        let work = self.work_dir();
        let ignore = self.work_dir().join("ignore");

        debug!("Checking mod files dir exists at {modfiles:?}");
        modfiles
            .try_exists()
            .map_err(|e| DirectoryCheckError::new(modfiles, e))?;

        debug!("Ensuring dl dir exists: {download:?}");
        create_dir_all(&download).map_err(|e| DirectoryCreationError::new(download, e))?;

        debug!("Ensuring mods dir exists: {mods:?}");
        create_dir_all(&mods).map_err(|e| DirectoryCreationError::new(mods, e))?;

        debug!("Ensuring upper exists: {overwrite:?}");
        create_dir_all(&overwrite).map_err(|e| DirectoryCreationError::new(overwrite, e))?;

        debug!("Ensuring work dir exists: {work:?}");
        create_dir_all(&work).map_err(|e| DirectoryCreationError::new(work, e))?;

        debug!("Ensuring ignore dir exists: {ignore:?}");
        create_dir_all(&ignore).map_err(|e| DirectoryCreationError::new(ignore, e))?;
        Ok(())
    }
}

impl GameTrait for Game {}
impl GameTrait for GameDomain {}

pub trait GameTraitMut: GameAttrMut {
    fn expand_path(&mut self, config: &Configuration) {
        let mut path = config.general.modding_root.clone();
        path.push(self.path());
        *self.path_mut() = shellexpand::path::tilde(&path).into();
    }
}

impl GameTraitMut for Game {}
impl GameTraitMut for GameDomain {}

pub trait GameDomainTrait {}

pub trait GameDomainTraitMut {}

pub trait GameAttr {
    fn name(&self) -> &str;
    fn path(&self) -> &Path;
    fn aliases(&self) -> &Vec<String>;
    fn steamid(&self) -> &u32;
}

impl GameAttr for Game {
    fn name(&self) -> &str {
        &self.name
    }
    fn path(&self) -> &Path {
        &self.path
    }
    fn aliases(&self) -> &Vec<String> {
        &self.aliases
    }
    fn steamid(&self) -> &u32 {
        &self.steamid
    }
}

impl GameAttr for GameDomain {
    fn name(&self) -> &str {
        &self.name
    }
    fn path(&self) -> &Path {
        &self.path
    }
    fn aliases(&self) -> &Vec<String> {
        &self.aliases
    }
    fn steamid(&self) -> &u32 {
        &self.steamid
    }
}

pub trait GameAttrMut: GameAttr {
    fn name_mut(&mut self) -> &mut String;
    fn path_mut(&mut self) -> &mut PathBuf;
    fn aliases_mut(&mut self) -> &mut Vec<String>;
    fn steamid_mut(&mut self) -> &mut u32;
}

impl GameAttrMut for Game {
    fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }
    fn path_mut(&mut self) -> &mut PathBuf {
        &mut self.path
    }
    fn aliases_mut(&mut self) -> &mut Vec<String> {
        &mut self.aliases
    }
    fn steamid_mut(&mut self) -> &mut u32 {
        &mut self.steamid
    }
}

impl GameAttrMut for GameDomain {
    fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }
    fn path_mut(&mut self) -> &mut PathBuf {
        &mut self.path
    }
    fn aliases_mut(&mut self) -> &mut Vec<String> {
        &mut self.aliases
    }
    fn steamid_mut(&mut self) -> &mut u32 {
        &mut self.steamid
    }
}

pub trait GameDomainAttr {
    fn domain(&self) -> &str;
}

impl GameDomainAttr for GameDomain {
    fn domain(&self) -> &str {
        &self.domain
    }
}

pub trait GameDomainAttrMut: GameDomainAttr {
    fn domain_mut(&mut self) -> &mut String;
}

impl GameDomainAttrMut for GameDomain {
    fn domain_mut(&mut self) -> &mut String {
        &mut self.domain
    }
}

impl TryFrom<Game> for GameDomain {
    type Error = ConfigError;
    fn try_from(g: Game) -> Result<Self> {
        match g.domain {
            Some(domain) => Ok(GameDomain {
                domain,
                name: g.name,
                path: g.path,
                aliases: g.aliases,
                steamid: g.steamid,
            }),
            None => Err(ConfigError::DomainMissingError { game: g.name })?,
        }
    }
}

// hack to unwrap games
// this enum lets us strip the 'game' key which holds the array of games in the toml file
// this works because serde will implicitly call .into() on the struct which will convert it to a Vec<Game>
#[derive(Deserialize)]
pub struct Games {
    #[serde(rename = "game")]
    games: Option<Vec<Game>>,
}

impl From<Games> for Vec<Game> {
    fn from(val: Games) -> Self {
        match val {
            Games { games: Some(games) } => games,
            // in the case we get an empty file, we return an empty list
            // without this, we get error out with a TOML parse error
            _ => vec![],
        }
    }
}

pub mod prelude {
    pub use super::{Game, GameDomain};

    pub use super::{GameAttr, GameAttrMut};
    pub use super::{GameTrait, GameTraitMut};

    pub use super::{GameDomainAttr, GameDomainAttrMut};
    pub use super::{GameDomainTrait, GameDomainTraitMut};
}
