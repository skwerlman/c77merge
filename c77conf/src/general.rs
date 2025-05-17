use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct GeneralConf {
    #[serde(deserialize_with = "deserialize_root")]
    pub modding_root: PathBuf,
    #[serde(deserialize_with = "deserialize_root")]
    pub steam_root: PathBuf,
}

// override modding_root deserialization to expand path
fn deserialize_root<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let path = PathBuf::from(s);
    Ok(shellexpand::path::tilde(&path).into())
}
