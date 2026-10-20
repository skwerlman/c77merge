use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
};

use serde::Deserialize;

use crate::file_listing::FileUpdate;

#[derive(Debug, Deserialize, Clone)]
pub struct Changelog(Vec<FileUpdate>);

impl Changelog {
    pub fn sort(&mut self) {
        self.0.sort();
    }

    /// Get a `FileUpdate` to the newest version of the specified file.
    ///
    /// Sorts the changelog as a side effect.
    pub fn collapse(&mut self, file_id: u32) -> FileUpdate {
        self.sort();
        let mut id = file_id;
        for &update in self.0.iter() {
            if update.old_file_id == id {
                id = update.new_file_id;
            }
        }
        FileUpdate {
            new_file_id: id,
            old_file_id: file_id,
        }
    }
}

impl Display for Changelog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[ ")?;
        for update in self.0.iter() {
            write!(f, "{update}, ")?;
        }
        write!(f, "]")?;
        Ok(())
    }
}

impl Deref for Changelog {
    type Target = Vec<FileUpdate>;

    fn deref(&self) -> &'_ Self::Target {
        &self.0
    }
}

impl DerefMut for Changelog {
    fn deref_mut(&mut self) -> &'_ mut Self::Target {
        &mut self.0
    }
}
