use std::io;

use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum C77Error {
    #[error(transparent)]
    #[diagnostic(code(c77merge::io_error))]
    HashError(#[from] io::Error),

    #[error("{0}")]
    Report(miette::Report),
}

impl From<miette::Report> for C77Error {
    fn from(report: miette::Report) -> Self {
        C77Error::Report(report)
    }
}

#[derive(Error, Debug, Diagnostic)]
#[error("Failed hashing some files")]
pub struct HashErrors {
    #[related]
    inner: Vec<C77Error>,
}

impl From<Vec<C77Error>> for HashErrors {
    fn from(inner: Vec<C77Error>) -> Self {
        HashErrors { inner }
    }
}

impl HashErrors {
    pub fn has_errors(&self) -> bool {
        !self.inner.is_empty()
    }
}
