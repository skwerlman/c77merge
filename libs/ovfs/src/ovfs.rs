use std::path::PathBuf;
use std::{fs::create_dir_all, path::Path};

use miette::{IntoDiagnostic, WrapErr};
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

    pub fn overlayfs_spec(&self) -> (&Path, &Vec<PathBuf>, &Path, &Path) {
        (
            &self.upper,
            &self.lowers,
            &self.work_directory,
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

    pub fn set_index(&mut self, index: usize) {
        self.work_directory = self.work_directory.join(format!("work-{index}"));
    }

    #[instrument(level = "trace", skip(self))]
    pub fn ensure_dirs(&self) -> Result<()> {
        debug!("Ensuring mountpoint exists: {}", self.mountpoint.display());
        create_dir_all(&self.mountpoint)
            .into_diagnostic()
            .wrap_err("Failed to create mountpoint")?;

        debug!("Ensuring workdir exists: {}", self.work_directory.display());
        create_dir_all(&self.work_directory)
            .into_diagnostic()
            .wrap_err("Failed to create workdir")?;
        create_dir_all(&self.upper)
            .into_diagnostic()
            .wrap_err("Failed to create upper directory")?;
        Ok(())
    }
}
