use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::Result;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ModProfile {
    pub profile: Profile,

    #[serde(rename = "mod", default)]
    pub mods: Vec<ModInfo>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Profile {
    pub name: String,
    pub description: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct ModInfo {
    pub id: u32,
    pub fid: u32,
    #[serde(deserialize_with = "crate::deserialize_path")]
    pub archive: PathBuf,
    #[serde(deserialize_with = "crate::deserialize_path")]
    pub unpacked: PathBuf,
    #[serde(default, skip_serializing_if = "crate::empty_path")]
    #[serde(deserialize_with = "crate::deserialize_path")]
    pub child_dir: PathBuf,
}

impl ModProfile {
    pub fn as_toml(&self) -> Result<String> {
        Ok(toml::to_string(self)?)
    }
}
