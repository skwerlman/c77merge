use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct ModID(u32);

impl From<u32> for ModID {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct ArchiveID(u32);

impl From<u32> for ArchiveID {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct C77Mod {
    name: String,
    id: ModID,
    archives: Vec<ArchiveID>,
}

impl C77Mod {
    pub fn new<T>(
        name: impl Into<Option<String>>,
        id: impl Into<Option<ModID>>,
        archives: impl Into<Option<Vec<ArchiveID>>>,
    ) -> Self {
        // Into<Option<String>> -> Option<String> -> String
        // Into<Option<String>> -> Option<String> -> None -> String::default()
        let name = name.into().unwrap_or_default();

        // Into<Option<ModID>> -> Option<ModID> -> ModID
        // Into<Option<ModID>> -> Option<ModID> -> None -> T::default() -> ModID
        let id = id.into().unwrap_or_default();

        // Into<Option<Vec<ArchiveID>>> -> Option<Vec<ArchiveID>> -> Vec<ArchiveID>
        // Into<Option<Vec<ArchiveID>>> -> Option<Vec<ArchiveID>> -> None -> Vec<ArchiveID>::default()
        let archives = archives.into().unwrap_or_default();

        Self { name, id, archives }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct ModArchive {
    id: ArchiveID,
    modid: ModID,
    version: String,
    archive_path: PathBuf,
    output_path: PathBuf,
}
