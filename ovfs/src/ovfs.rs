use std::fs::create_dir_all;
use std::path::PathBuf;

use tracing::{debug, instrument, warn};

use crate::Result;

#[derive(Debug, Clone)]
pub struct OVFSTree {
    mountpoint: PathBuf,
    work_directory: PathBuf,
    upper: PathBuf,
    lowers: Vec<PathBuf>,
}

impl OVFSTree {
    pub fn new(
        mountpoint: PathBuf,
        work_directory: PathBuf,
        overwrite: PathBuf,
        lowers: Vec<PathBuf>,
    ) -> Self {
        Self {
            mountpoint,
            work_directory,
            upper: overwrite,
            lowers,
        }
    }

    pub fn work_directory_with_index(&self, index: usize) -> PathBuf {
        self.work_directory.join(format!("work-{index}"))
    }

    pub fn overlayfs_spec(&self, index: usize) -> (&PathBuf, &Vec<PathBuf>, PathBuf, &PathBuf) {
        (
            &self.upper,
            &self.lowers,
            self.work_directory_with_index(index),
            &self.mountpoint,
        )
    }

    pub fn with_lower(&mut self, lower: PathBuf) {
        if let Ok(true) = lower.try_exists() {
            self.lowers.push(lower);
        } else {
            warn!("Lower directory does not exist: {}", lower.display());
            warn!("Ignore this if the missing directory is inside a previous overlay");
        }
    }

    #[instrument(level = "trace", skip(self, index))]
    pub fn ensure_dirs(&self, index: usize) -> Result<()> {
        debug!("Ensuring mountpoint exists: {}", self.mountpoint.display());
        create_dir_all(&self.mountpoint)?;

        let workdir = self.work_directory.join(format!("work-{index}"));
        debug!("Ensuring workdir exists: {}", workdir.display());
        create_dir_all(&workdir)?;
        create_dir_all(&self.upper)?;
        Ok(())
    }
}
