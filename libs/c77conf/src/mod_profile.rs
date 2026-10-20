use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::Result;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ModProfile {
    pub profile: Profile,

    #[serde(rename = "mod", default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mods: Vec<ModInfo>,

    #[serde(rename = "mapping", default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mappings: Vec<Mapping>,
}

impl ModProfile {
    pub fn as_toml(&self) -> Result<String> {
        Ok(toml::to_string(self)?)
    }
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
    pub mod_name: Option<String>,
    pub file_name: Option<String>,
    #[serde(deserialize_with = "crate::deserializers::deserialize_optional_path")]
    pub archive: Option<PathBuf>,
    #[serde(deserialize_with = "crate::deserializers::deserialize_optional_path")]
    pub unpacked: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "crate::deserializers::empty_path")]
    #[serde(deserialize_with = "crate::deserializers::deserialize_path")]
    pub child_dir: PathBuf,
}

impl ModInfo {
    /// Create a new `ModInfo` with a specified `file_id`, used when updating mods.
    pub fn clone_to_new_with_fid(&self, file_id: u32) -> Self {
        let mut new = self.clone();
        new.fid = file_id;
        new
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Mapping {
    pub real: PathBuf,
    pub mapped: PathBuf,
    #[serde(default)]
    pub early: bool,
}
