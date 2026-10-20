use http::StatusCode;
use miette::{Diagnostic, NamedSource, SourceSpan};
use reqwest_middleware::ClientWithMiddleware;
use serde::Deserialize;
use thiserror::Error;
use tracing::{instrument, trace};

use crate::API_BASE;
use crate::client::{NexusClient, RATELIMITED, VALIDDOMAIN};
use crate::error::{DeserializeError, json_err};
use crate::file_listing::File;

type Result = std::result::Result<Md5LookupData, Md5LookupError>;

#[derive(Deserialize, Debug, Clone)]
pub struct Mod {
    pub name: String,
    // pub summary: String,
    // pub description: String,
    pub mod_id: u32,
    pub domain_name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Md5LookupData {
    #[serde(rename = "mod")]
    pub mod_details: Mod,
    pub file_details: File,
}

#[derive(Debug, Error, Diagnostic)]
#[error("Error looking up mod by MD5")]
#[diagnostic(code(nexusapi::md5lookup))]
pub struct Md5LookupError {
    #[diagnostic_source]
    caused_by: InnerMd5LookupError,
}

impl<T> From<T> for Md5LookupError
where
    T: Into<InnerMd5LookupError>,
{
    fn from(value: T) -> Self {
        Self {
            caused_by: value.into(),
        }
    }
}

#[derive(Debug, Error, Diagnostic)]
#[diagnostic()]
enum InnerMd5LookupError {
    #[error("http client error: {0}")]
    #[diagnostic(code(nexusapi::md5lookup::http::reqwest))]
    HTTPClientError(#[from] reqwest::Error),
    #[error("http middleware error: {0}")]
    #[diagnostic(code(nexusapi::md5lookup::http::reqwest::middleware))]
    HTTPMiddlewareError(#[from] reqwest_middleware::Error),
    #[error("deserialization error")]
    #[diagnostic(code(nexusapi::md5lookup::serde::bad_json))]
    DeserializationError {
        #[source]
        cause: serde_json::Error,
        #[source_code]
        source: NamedSource<String>,
        #[label]
        bad_part: SourceSpan,
    },
    #[error("Mod not found")]
    #[diagnostic(code(nexusapi::not_found))]
    NotFound,
    #[error("Invalid hash")]
    #[diagnostic(code(nexusapi::md5lookup::invalid_hash))]
    InvalidHash,
    #[error("Unexpected status code: {0}")]
    #[diagnostic(code(nexusapi::md5lookup::http::unexpected_status))]
    UnexpectedStatusCode(StatusCode),
    #[error("Something went wrong finding the maximum file id!")]
    #[diagnostic(code(nexusapi::md5lookup::no_max_id))]
    NoMaximumFileId,
}

impl DeserializeError for InnerMd5LookupError {
    type Error = serde_json::Error;
    type Source = String;
    type Output = Self;
    fn deserialize_error(
        error: Self::Error,
        source: NamedSource<Self::Source>,
        span: SourceSpan,
    ) -> Self::Output {
        InnerMd5LookupError::DeserializationError {
            cause: error,
            source,
            bad_part: span,
        }
    }
}

#[instrument(level = "trace", skip(client))]
pub async fn get_by_hash(client: NexusClient<RATELIMITED, VALIDDOMAIN>, hash: String) -> Result {
    let mut route = API_BASE.to_owned();
    route.push_str("v1/games/");
    route.push_str(&client.game_domain);
    route.push_str("/mods/md5_search/");
    route.push_str(hash.as_str());
    route.push_str(".json");

    let c: ClientWithMiddleware = client.into();

    let resp = c.get(&route).send().await?;

    if resp.status() == StatusCode::NOT_FOUND {
        return Err(InnerMd5LookupError::NotFound.into());
    } else if resp.status() == StatusCode::UNPROCESSABLE_ENTITY {
        return Err(InnerMd5LookupError::InvalidHash.into());
    } else if !resp.status().is_success() {
        return Err(InnerMd5LookupError::UnexpectedStatusCode(resp.status()).into());
    }

    let text = resp.text().await?;

    let lookups: Vec<Md5LookupData> =
        serde_json::from_str(&text).map_err(|e| json_err::<InnerMd5LookupError>(e, route, text))?;

    trace!("lookup results: {:?}", lookups);

    let mut lookup: Option<Md5LookupData> = None;

    for l in lookups {
        if let Some(ref newest) = lookup {
            if newest.file_details.file_id < l.file_details.file_id {
                trace!("taking newer: {l:?}");
                lookup = Some(l);
            } else {
                trace!("keeping: {newest:?}");
            }
        } else {
            trace!("starting with: {l:?}");
            lookup = Some(l);
        }
    }

    lookup.ok_or(InnerMd5LookupError::NoMaximumFileId.into())
}
