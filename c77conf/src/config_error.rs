use std::env::VarError;

use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ConfigError {
    #[error("missing environment variable: {0:?}")]
    MissingEnv(VarError),
    #[error("config error: {0}")]
    StrError(String),
    #[error("io error: {0}")]
    IOError(std::io::Error),
    #[error("toml error: {0}")]
    TomlDeError(toml::de::Error),
    #[error("toml error: {0}")]
    TomlSerError(toml::ser::Error),
    #[error("config error: {0}")]
    ConfigError(config::ConfigError),
}

impl<'a> From<&'a str> for ConfigError {
    fn from(value: &'a str) -> Self {
        ConfigError::StrError(value.to_owned())
    }
}

impl From<config::ConfigError> for ConfigError {
    fn from(value: config::ConfigError) -> Self {
        ConfigError::ConfigError(value)
    }
}

impl From<VarError> for ConfigError {
    fn from(value: VarError) -> Self {
        ConfigError::MissingEnv(value)
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(value: std::io::Error) -> Self {
        ConfigError::IOError(value)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(value: toml::de::Error) -> Self {
        ConfigError::TomlDeError(value)
    }
}

impl From<toml::ser::Error> for ConfigError {
    fn from(value: toml::ser::Error) -> Self {
        ConfigError::TomlSerError(value)
    }
}
