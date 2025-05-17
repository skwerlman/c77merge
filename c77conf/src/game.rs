use std::path::PathBuf;

use serde::Deserialize;

use crate::Configuration;

#[derive(Deserialize, Debug, Clone)]
pub struct Game {
    pub name: String,
    pub path: PathBuf,
    pub aliases: Vec<String>,
    pub domain: String,
    pub steamid: u32,
}

impl Game {
    pub fn expand_path(&mut self, config: &Configuration) -> &mut Self {
        let mut path = config.general.modding_root.clone();
        path.push(&self.path);
        self.path = shellexpand::path::tilde(&path).into();
        self
    }

    pub fn download_dir(&self) -> PathBuf {
        self.path.join("Download")
    }
    pub fn mods_dir(&self) -> PathBuf {
        self.path.join("Mods")
    }
    pub fn overwrite_dir(&self) -> PathBuf {
        self.path.join("Overwrite")
    }

    pub fn profile(&self, name: &str) -> PathBuf {
        self.path.join(name).with_extension("toml")
    }
}

// hack to unwrap games
// this enum lets us strip the 'game' key which holds the array of games in the toml file
// this works because serde will implicitly call .into() on the enum which will convert it to a Vec<Game>
// using the From<Games> impl which just strips off the outer enum
#[derive(Deserialize)]
pub enum Games {
    #[serde(rename = "game")]
    Games(Vec<Game>),
}

impl From<Games> for Vec<Game> {
    fn from(val: Games) -> Self {
        match val {
            Games::Games(games) => games,
        }
    }
}
