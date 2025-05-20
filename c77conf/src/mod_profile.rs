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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ModInfo {
    pub id: u32,
    pub fid: u32,
    pub archive: String,
    pub unpacked: String,
}

impl ModProfile {
    pub fn as_toml(&self) -> Result<String> {
        Ok(toml::to_string(self)?)
    }
}
