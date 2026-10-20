use std::io;

use miette::Diagnostic;
use thiserror::Error;

use crate::token::AcfToken;

pub(crate) type Result<T, E = AcfError> = std::result::Result<T, E>;

#[derive(Debug, Error, Diagnostic)]
#[error("ACF Error")]
pub enum AcfError {
    StreamError(
        #[from]
        #[diagnostic_source]
        StreamError,
    ),
    AcfReadError(
        #[from]
        #[diagnostic_source]
        AcfReadError,
    ),
    ParseError(
        #[from]
        #[diagnostic_source]
        ParseError,
    ),
}

#[derive(Debug, Error, Diagnostic)]
pub enum StreamError {
    #[error("Invalid data: {error}")]
    Parse {
        #[from]
        #[diagnostic_source]
        error: ParseError,
    },
    #[error("Unexpected Token: {token:?}")]
    UnexpectedToken { token: AcfToken },
    #[error("Path not found: {}", format_path(&.0[..]))]
    PathNotFound(Vec<String>),
}

#[derive(Debug, Error, Diagnostic)]
#[error("Error reading ACF file")]
pub struct AcfReadError {
    #[source]
    cause: io::Error,
    #[help]
    help: Option<String>,
}

impl From<io::Error> for AcfReadError {
    fn from(value: io::Error) -> Self {
        AcfReadError {
            cause: value,
            help: None,
        }
    }
}

impl AcfReadError {
    pub(crate) fn append_help<S>(mut self, help: S) -> Self
    where
        S: Into<String>,
    {
        if let Some(h) = self.help.as_mut() {
            h.push('\n');
            h.push_str(&help.into());
        } else {
            self.help = Some(help.into());
        };
        self
    }
}

#[derive(Debug, Error, Diagnostic)]
pub enum ParseError {
    #[error("Generic I/O error: {0}")]
    Io(
        #[from]
        #[source]
        io::Error,
    ),
    #[error("Unexpected Character '{0:?}'")]
    UnexpectedCharacter(char),
    #[error("Unterminated String literal")]
    UnterminatedString,
    #[error("Unexpected EOF")]
    UnexpectedEof,
}

fn format_path(items: impl IntoIterator<Item = impl AsRef<str>>) -> String {
    let mut s = String::new();
    for p in items {
        s.push('.');
        s.push_str(p.as_ref());
    }
    s
}
