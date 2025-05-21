use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct NexusConf {
    pub api_key: String,
    pub forbid_http: bool,
    pub verbose_connection: bool,
    pub enable_compression: bool,
    pub connect_timeout: u64,
    pub read_timeout: u64,
    pub timeout: u64,
    #[serde(deserialize_with = "crate::deserialize_path")]
    pub cache_path: PathBuf,
}
