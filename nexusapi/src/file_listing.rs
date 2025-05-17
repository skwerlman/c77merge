use tracing::warn;

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct File {
    pub file_id: u32,
    pub file_name: String,
    pub size_in_bytes: u64,
    #[serde(rename = "name")]
    pub mod_name: String,
    pub version: String,
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct FileUpdate {
    old_file_id: u32,
    new_file_id: u32,
}

#[derive(Debug, Deserialize)]
pub struct FileListing {
    files: Vec<File>,
    file_updates: Vec<FileUpdate>,
}

impl FileListing {
    pub fn next(&self, file_id: u32) -> Option<&File> {
        let update = self
            .file_updates
            .iter()
            .find(|update| update.old_file_id == file_id);

        if update.is_none() {
            if let Some(file) = self.files.iter().find(|file| file.file_id == file_id) {
                // valid file, but no updates
                return Some(file);
            }
            // no information about the file
            warn!("File with ID {} not found", file_id);
            return None;
        }

        // found update for file
        self.files
            .iter()
            .find(|file| file.file_id == update.unwrap().new_file_id)
    }
}
