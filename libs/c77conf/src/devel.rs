use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct DevelConf {
    #[serde(deserialize_with = "crate::deserializers::deserialize_path")]
    pub testing_root: PathBuf,
}
