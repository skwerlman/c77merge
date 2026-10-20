use std::{env::VarError, io, path::PathBuf};

use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
#[error("default text")]
#[diagnostic()]
pub enum ConfigError {
    #[error("couldn't read environment variable: {var}")]
    MissingEnv {
        #[source]
        cause: VarError,
        var: &'static str,
    },
    // #[error("Error reading configuration: {0:?}")]
    // StrError(String),
    #[error("Syntax error")]
    TomlSyntaxError(#[source] toml_edit::TomlError),
    #[error("Config schema error")]
    TomlSchemaError(#[source] toml_edit::de::Error),
    #[error(transparent)]
    TomlSerError(#[from] toml::ser::Error),
    #[error(transparent)]
    ConfigError(#[from] config::ConfigError),
    #[error(transparent)]
    ConfigOpenError(#[from] ConfigOpenError),
    #[error(transparent)]
    DirectoryCreationError(#[from] DirectoryCreationError),
    #[error(transparent)]
    DirectoryCheckError(#[from] DirectoryCheckError),
    #[error("No domain key found for {game}")]
    DomainMissingError { game: String },
}

#[derive(Error, Debug, Diagnostic)]
#[error("Error reading configuration file: {file}")]
#[diagnostic()]
pub struct ConfigOpenError {
    file: PathBuf,
    #[source]
    cause: io::Error,
}

impl ConfigOpenError {
    pub fn new(file: PathBuf, cause: io::Error) -> Self {
        ConfigOpenError { file, cause }
    }
}

#[derive(Error, Debug, Diagnostic)]
#[error("Error creating directory: {directory}")]
#[diagnostic()]
pub struct DirectoryCreationError {
    directory: PathBuf,
    #[source]
    cause: io::Error,
}

impl DirectoryCreationError {
    pub fn new(directory: PathBuf, cause: io::Error) -> Self {
        DirectoryCreationError { directory, cause }
    }
}

#[derive(Error, Debug, Diagnostic)]
#[error("Error checking directory: {directory}")]
#[diagnostic()]
pub struct DirectoryCheckError {
    directory: PathBuf,
    #[source]
    cause: io::Error,
}

impl DirectoryCheckError {
    pub fn new(directory: PathBuf, cause: io::Error) -> Self {
        DirectoryCheckError { directory, cause }
    }
}
