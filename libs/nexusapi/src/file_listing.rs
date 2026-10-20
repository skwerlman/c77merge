use std::fmt::Display;

use http::StatusCode;
use miette::{Diagnostic, NamedSource, SourceSpan};
use regex::bytes::Regex;
use reqwest_middleware::ClientWithMiddleware;
use thiserror::Error;
use tracing::{instrument, warn};

use serde::{Deserialize, Deserializer, de};

use crate::API_BASE;
use crate::changelog::Changelog;
use crate::client::{NexusClient, RATELIMITED, VALIDDOMAIN};
use crate::error::{DeserializeError, json_err};

type Result<T> = std::result::Result<T, FileListingError>;

#[derive(Debug, Deserialize, Clone)]
pub struct File {
    pub file_id: u32,
    pub file_name: String,
    // for some reason, this can be null
    pub size_in_bytes: Option<u64>,
    #[serde(rename = "name")]
    pub mod_name: String,
    pub version: String,
    #[serde(
        rename = "external_virus_scan_url",
        deserialize_with = "hack_get_sha1_from_virus_scan"
    )]
    pub sha1: Option<String>,
}

fn hack_get_sha1_from_virus_scan<'de, D>(de: D) -> std::result::Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let r = Regex::new(r"[0-9a-f]{64}")
        .map_err(|inner| de::Error::custom(format!("regex compilation error: {inner}")))?;

    if let Ok(url) = String::deserialize(de)
        && url.starts_with("https://www.virustotal.com/gui/file/")
        && r.is_match(&url.as_bytes()[36..100])
    {
        Ok(Some(url[36..100].to_owned()))
    } else {
        Ok(None)
    }
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileUpdate {
    pub new_file_id: u32,
    pub old_file_id: u32,
}

impl FileUpdate {
    pub fn is_change(&self) -> bool {
        self.new_file_id != self.old_file_id
    }
}

impl Display for FileUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FileUpdate {{ {:?} ==> {:?} }}",
            self.old_file_id, self.new_file_id
        )?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct FileListing {
    files: Vec<File>,
    #[serde(rename = "file_updates")]
    changelog: Changelog,
}

impl FileListing {
    pub fn next(&self, file_id: u32) -> Option<&File> {
        let update = self
            .changelog
            .iter()
            .find(|update| update.old_file_id == file_id);

        if update.is_none() {
            if let Some(file) = self.files.iter().find(|file| file.file_id == file_id) {
                // valid file, but no updates
                return Some(file);
            }
            // no information about the file
            warn!("File with ID {file_id} not found");
            return None;
        }

        // found update for file
        self.files
            .iter()
            .find(|file| file.file_id == update.unwrap().new_file_id)
    }

    pub fn changelog(&self) -> &Changelog {
        &self.changelog
    }

    pub fn changelog_mut(&mut self) -> &mut Changelog {
        &mut self.changelog
    }

    pub fn files(&self) -> Vec<&File> {
        self.files.iter().collect()
    }

    pub fn files_cloned(&self) -> Vec<File> {
        self.files.to_vec()
    }
}

#[instrument(level = "trace", skip(client))]
pub async fn get_file_listing(
    client: NexusClient<RATELIMITED, VALIDDOMAIN>,
    mod_id: u32,
) -> Result<FileListing> {
    let mut route = API_BASE.to_owned();
    route.push_str("v1/games/");
    route.push_str(&client.game_domain);
    route.push_str("/mods/");
    route.push_str(&mod_id.to_string());
    route.push_str("/files.json");

    let c: ClientWithMiddleware = client.into();
    let resp = c.get(&route).send().await?;

    if resp.status() == StatusCode::NOT_FOUND {
        return Err(InnerFileListingError::ModNotFound.into());
    } else if !resp.status().is_success() {
        return Err(InnerFileListingError::UnexpectedStatusCode(resp.status()).into());
    }

    let text = resp.text().await?;

    let listing = serde_json::from_str(&text)
        .map_err(|e| json_err::<InnerFileListingError>(e, route, text))?;

    Ok(listing)
}

#[instrument(level = "trace", skip(client))]
pub async fn get_file_by_id(
    client: NexusClient<RATELIMITED, VALIDDOMAIN>,
    mod_id: u32,
    file_id: u32,
) -> Result<File> {
    let mut route = API_BASE.to_owned();
    route.push_str("v1/games/");
    route.push_str(&client.game_domain);
    route.push_str("/mods/");
    route.push_str(&mod_id.to_string());
    route.push_str("/files/");
    route.push_str(&file_id.to_string());
    route.push_str(".json");

    let c: ClientWithMiddleware = client.into();
    let resp = c.get(&route).send().await?;

    if resp.status() == StatusCode::NOT_FOUND {
        return Err(InnerFileListingError::FileNotFound.into());
    } else if !resp.status().is_success() {
        return Err(InnerFileListingError::UnexpectedStatusCode(resp.status()).into());
    }

    let text = resp.text().await?;

    let listing = serde_json::from_str(&text)
        .map_err(|e| json_err::<InnerFileListingError>(e, route, text))?;

    Ok(listing)
}

#[derive(Debug, Error, Diagnostic)]
#[error("Error looking up mod by MD5:")]
#[diagnostic(code(nexusapi::file_listing))]
pub struct FileListingError {
    #[source]
    #[diagnostic_source]
    caused_by: InnerFileListingError,
}

impl<T> From<T> for FileListingError
where
    T: Into<InnerFileListingError>,
{
    fn from(value: T) -> Self {
        Self {
            caused_by: value.into(),
        }
    }
}

#[derive(Debug, Error, Diagnostic)]
enum InnerFileListingError {
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
    #[error("Mod not found")]
    #[diagnostic(code(nexusapi::file_listing::mod_not_found))]
    ModNotFound,
    #[error("Unexpected status code: {0}")]
    #[diagnostic(code(nexusapi::file_listing::http::unexpected_status))]
    UnexpectedStatusCode(StatusCode),
}

impl DeserializeError for InnerFileListingError {
    type Error = serde_json::Error;
    type Source = String;
    type Output = Self;
    fn deserialize_error(
        error: Self::Error,
        source: NamedSource<Self::Source>,
        span: SourceSpan,
    ) -> Self::Output {
        InnerFileListingError::DeserializationError {
            cause: error,
            source,
            bad_part: span,
        }
    }
}
