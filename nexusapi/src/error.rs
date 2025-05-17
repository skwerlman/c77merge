use miette::Diagnostic;
use serde::Deserialize;
use thiserror::Error;

#[derive(Error, Debug, Deserialize, Diagnostic)]
pub enum Error {
    #[error("nexus api error: {0:?}")]
    NexusError(String),
    #[error("http client error: {0}")]
    #[serde(skip)]
    HTTPClientError(reqwest::Error),
    #[error("http middleware error: {0}")]
    #[serde(skip)]
    HTTPMiddlewareError(reqwest_middleware::Error),
    #[error("header value error: {0}")]
    #[serde(skip)]
    HeaderValueError(reqwest::header::InvalidHeaderValue),
    #[error("deserialization error: {0}")]
    #[serde(skip)]
    DeserializationError(serde_json::Error),
    #[error("tokio join error: {0}")]
    #[serde(skip)]
    TokioJoinError(tokio::task::JoinError),
}

impl From<&'static str> for Error {
    fn from(err: &'static str) -> Self {
        err.to_owned().into()
    }
}

impl From<String> for Error {
    fn from(err: String) -> Self {
        Error::NexusError(err)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::HTTPClientError(err)
    }
}

impl From<reqwest_middleware::Error> for Error {
    fn from(err: reqwest_middleware::Error) -> Self {
        Error::HTTPMiddlewareError(err)
    }
}

impl From<reqwest::header::InvalidHeaderValue> for Error {
    fn from(err: reqwest::header::InvalidHeaderValue) -> Self {
        Error::HeaderValueError(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::DeserializationError(err)
    }
}

impl From<tokio::task::JoinError> for Error {
    fn from(err: tokio::task::JoinError) -> Self {
        Error::TokioJoinError(err)
    }
}
