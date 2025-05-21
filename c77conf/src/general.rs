use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct GeneralConf {
    #[serde(deserialize_with = "crate::deserialize_path")]
    pub modding_root: PathBuf,
    #[serde(deserialize_with = "crate::deserialize_path")]
    pub steam_root: PathBuf,
}
