use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct GeneralConf {
    #[serde(deserialize_with = "deserialize_path")]
    pub modding_root: PathBuf,
    #[serde(deserialize_with = "deserialize_path")]
    pub steam_root: PathBuf,
}

// override deserialization to expand string to absolute path
fn deserialize_path<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let path = PathBuf::from(s);
    Ok(shellexpand::path::tilde(&path).into())
}
