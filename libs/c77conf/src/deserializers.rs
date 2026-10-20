use std::path::{Path, PathBuf};

use serde::Deserialize as _;

pub(crate) fn deserialize_path<'de, D>(deserializer: D) -> std::result::Result<PathBuf, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let path = PathBuf::from(s);
    Ok(shellexpand::path::tilde(&path).into())
}

pub(crate) fn deserialize_optional_path<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<PathBuf>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = Option::<String>::deserialize(deserializer)?;
    let path = s.map(PathBuf::from);
    Ok(path.map(|p| shellexpand::path::tilde(&p).into()))
}

pub(crate) fn empty_path(path: impl AsRef<Path>) -> bool {
    path.as_ref().as_os_str().is_empty()
}
