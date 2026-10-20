#![feature(min_adt_const_params)]

pub(crate) mod client;
pub(crate) mod middleware;
pub(crate) mod ratelimiter;

pub mod changelog;
pub mod download_link;
pub mod error;
pub mod file_listing;
pub mod md5lookup;
pub mod user;

use crate::client::{NexusClient, RATELIMITED, VALIDDOMAIN};

pub(crate) const API_BASE: &str = "https://api.nexusmods.com/";

pub type Result<T> = std::result::Result<T, error::Error>;
pub type ApiClient = NexusClient<RATELIMITED, VALIDDOMAIN>;

// public apis
pub use client::login;
pub use download_link::get_download_link;
pub use file_listing::{get_file_by_id, get_file_listing};
pub use md5lookup::get_by_hash;

use std::{fmt::Debug, path::Path, sync::Arc};

pub trait ClientConfig: Clone + Debug {
    fn cache_path(&self) -> &Path;
    fn api_key(&self) -> &str;
    fn forbid_http(&self) -> bool;
    fn timeout(&self) -> u64;
    fn connect_timeout(&self) -> u64;
    fn read_timeout(&self) -> u64;
    fn verbose_connection(&self) -> bool;
    fn enable_compression(&self) -> bool;
}

impl<T> ClientConfig for Arc<T>
where
    T: ClientConfig,
{
    fn cache_path(&self) -> &Path {
        self.as_ref().cache_path()
    }
    fn api_key(&self) -> &str {
        self.as_ref().api_key()
    }
    fn forbid_http(&self) -> bool {
        self.as_ref().forbid_http()
    }
    fn timeout(&self) -> u64 {
        self.as_ref().timeout()
    }
    fn connect_timeout(&self) -> u64 {
        self.as_ref().connect_timeout()
    }
    fn read_timeout(&self) -> u64 {
        self.as_ref().read_timeout()
    }
    fn verbose_connection(&self) -> bool {
        self.as_ref().verbose_connection()
    }
    fn enable_compression(&self) -> bool {
        self.as_ref().enable_compression()
    }
}
