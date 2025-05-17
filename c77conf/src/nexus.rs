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
    #[serde(deserialize_with = "deserialize_nexus_cache")]
    pub cache_path: PathBuf,
}

// override cache_path deserialization to expand path
fn deserialize_nexus_cache<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let path = PathBuf::from(s);
    Ok(shellexpand::path::tilde(&path).into())
}
