use miette::{Diagnostic, NamedSource, SourceOffset, SourceSpan};
use serde::Deserialize;
use thiserror::Error;

#[derive(Error, Debug, Deserialize, Diagnostic)]
pub enum Error {
    #[error("nexus api error: {0:?}")]
    #[diagnostic(code(nexusapi::api_error))]
    NexusError(String),
    #[error("http client error: {0}")]
    #[serde(skip)]
    #[diagnostic(code(nexusapi::http::reqwest))]
    HTTPClientError(#[from] reqwest::Error),
    #[error("http middleware error: {0}")]
    #[serde(skip)]
    #[diagnostic(code(nexusapi::http::reqwest::middleware))]
    HTTPMiddlewareError(#[from] reqwest_middleware::Error),
    #[error("header value error: {0}")]
    #[serde(skip)]
    #[diagnostic(code(nexusapi::http::bad_header))]
    HeaderValueError(#[from] reqwest::header::InvalidHeaderValue),
    #[error("deserialization error: {0}")]
    #[serde(skip)]
    #[diagnostic(code(nexusapi::serde::bad_json))]
    DeserializationError(#[from] serde_json::Error),
    #[error("tokio join error: {0}")]
    #[serde(skip)]
    #[diagnostic(code(nexusapi::tokio::runtime))]
    TokioJoinError(#[from] tokio::task::JoinError),
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

pub(crate) fn json_err<
    T: DeserializeError<Output = T, Error = serde_json::Error, Source = String>,
>(
    error: serde_json::Error,
    route: String,
    text: String,
) -> T {
    let line = error.line();
    let col = error.column();

    let location = SourceOffset::from_location(&text, line, col);

    T::deserialize_error(error, NamedSource::new(route, text), (location, 1).into())
}

pub(crate) trait DeserializeError {
    type Error;
    type Source;
    type Output;
    fn deserialize_error(
        error: Self::Error,
        source: NamedSource<Self::Source>,
        span: SourceSpan,
    ) -> Self::Output
    where
        <Self as DeserializeError>::Source: miette::SourceCode;
}
