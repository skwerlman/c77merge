use http::StatusCode;
use miette::{Diagnostic, NamedSource, SourceSpan};
use reqwest_middleware::ClientWithMiddleware;
use thiserror::Error;
use tracing::instrument;

use serde::Deserialize;

use crate::API_BASE;
use crate::client::{NexusClient, RATELIMITED, VALIDDOMAIN};
use crate::error::{DeserializeError, json_err};

type Result<T> = std::result::Result<T, DownloadLinksError>;

#[derive(Debug, Deserialize, Clone)]
pub struct DownloadLinks(Vec<DownloadLink>);

impl DownloadLinks {
    pub fn get_link_by_shortname<S: AsRef<str>>(&self, short_name: S) -> Option<&str> {
        let short_name = short_name.as_ref();

        let mut link = None;
        for link_info in self.0.iter() {
            if link_info.short_name == short_name {
                link = Some(link_info.uri.as_str());
                break;
            }
        }
        link
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DownloadLink {
    // name: String,
    short_name: String,
    #[serde(rename = "URI")]
    uri: String,
}

impl AsRef<str> for DownloadLink {
    fn as_ref(&self) -> &str {
        &self.uri
    }
}

#[instrument(level = "trace", skip(client))]
pub async fn get_download_link(
    client: NexusClient<RATELIMITED, VALIDDOMAIN>,
    mod_id: u32,
    file_id: u32,
) -> Result<DownloadLinks> {
    let mut route = API_BASE.to_owned();
    route.push_str("v1/games/");
    route.push_str(&client.game_domain);
    route.push_str("/mods/");
    route.push_str(&mod_id.to_string());
    route.push_str("/files/");
    route.push_str(&file_id.to_string());
    route.push_str("/download_link.json");

    let c: ClientWithMiddleware = client.into();
    let resp = c.get(&route).send().await?;

    if resp.status() == StatusCode::NOT_FOUND {
        return Err(InnerDownloadLinksError::FileNotFound.into());
    } else if !resp.status().is_success() {
        return Err(InnerDownloadLinksError::UnexpectedStatusCode(resp.status()).into());
    }

    let text = resp.text().await?;

    let listing = serde_json::from_str(&text)
        .map_err(|e| json_err::<InnerDownloadLinksError>(e, route, text))?;

    Ok(listing)
}

#[derive(Debug, Error, Diagnostic)]
#[error("Error getting download links:")]
#[diagnostic(code(nexusapi::file_listing))]
pub struct DownloadLinksError {
    #[source]
    #[diagnostic_source]
    caused_by: InnerDownloadLinksError,
}

impl<T> From<T> for DownloadLinksError
where
    T: Into<InnerDownloadLinksError>,
{
    fn from(value: T) -> Self {
        Self {
            caused_by: value.into(),
        }
    }
}

#[derive(Debug, Error, Diagnostic)]
enum InnerDownloadLinksError {
    #[error("http client error: {0}")]
    #[diagnostic(code(nexusapi::file_listing::http::reqwest))]
    HTTPClientError(#[from] reqwest::Error),
    #[error("http middleware error: {0}")]
    #[diagnostic(code(nexusapi::file_listing::http::reqwest::middleware))]
    HTTPMiddlewareError(#[from] reqwest_middleware::Error),
    #[error("deserialization error")]
    #[diagnostic(code(nexusapi::file_listing::serde::bad_json))]
    DeserializationError {
        #[source]
        cause: serde_json::Error,
        #[source_code]
        source: NamedSource<String>,
        #[label]
        bad_part: SourceSpan,
    },
    #[error("File not found")]
    #[diagnostic(code(nexusapi::file_listing::file_not_found))]
    FileNotFound,
    // #[error("Mod not found")]
    // #[diagnostic(code(nexusapi::file_listing::mod_not_found))]
    // ModNotFound,
    #[error("Unexpected status code: {0}")]
    #[diagnostic(code(nexusapi::file_listing::http::unexpected_status))]
    UnexpectedStatusCode(StatusCode),
}

impl DeserializeError for InnerDownloadLinksError {
    type Error = serde_json::Error;
    type Source = String;
    type Output = Self;
    fn deserialize_error(
        error: Self::Error,
        source: NamedSource<Self::Source>,
        span: SourceSpan,
    ) -> Self::Output {
        InnerDownloadLinksError::DeserializationError {
            cause: error,
            source,
            bad_part: span,
        }
    }
}
