use std::fmt::Debug;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct NexusConf {
    pub api_key: String,
    pub forbid_http: bool,
    pub verbose_connection: bool,
    pub enable_compression: bool,
    pub connect_timeout: u64,
    pub read_timeout: u64,
    pub timeout: u64,
    #[serde(deserialize_with = "crate::deserializers::deserialize_path")]
    pub cache_path: PathBuf,
}

impl Debug for NexusConf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NexusConf")
            .field("api_key", &"XXXXX")
            .field("forbid_http", &self.forbid_http)
            .field("verbose_connection", &self.verbose_connection)
            .field("enable_compression", &self.enable_compression)
            .field("connect_timeout", &self.connect_timeout)
            .field("read_timeout", &self.read_timeout)
            .field("timeout", &self.timeout)
            .field("cache_path", &self.cache_path)
            .finish()
    }
}

impl nexusapi::ClientConfig for NexusConf {
    fn api_key(&self) -> &str {
        &self.api_key
    }
    fn forbid_http(&self) -> bool {
        self.forbid_http
    }
    fn verbose_connection(&self) -> bool {
        self.verbose_connection
    }
    fn enable_compression(&self) -> bool {
        self.enable_compression
    }
    fn connect_timeout(&self) -> u64 {
        self.connect_timeout
    }
    fn read_timeout(&self) -> u64 {
        self.read_timeout
    }
    fn timeout(&self) -> u64 {
        self.timeout
    }
    fn cache_path(&self) -> &std::path::Path {
        &self.cache_path
    }
}

impl nexusapi::ClientConfig for &NexusConf {
    fn api_key(&self) -> &str {
        &self.api_key
    }
    fn forbid_http(&self) -> bool {
        self.forbid_http
    }
    fn verbose_connection(&self) -> bool {
        self.verbose_connection
    }
    fn enable_compression(&self) -> bool {
        self.enable_compression
    }
    fn connect_timeout(&self) -> u64 {
        self.connect_timeout
    }
    fn read_timeout(&self) -> u64 {
        self.read_timeout
    }
    fn timeout(&self) -> u64 {
        self.timeout
    }
    fn cache_path(&self) -> &std::path::Path {
        &self.cache_path
    }
}
