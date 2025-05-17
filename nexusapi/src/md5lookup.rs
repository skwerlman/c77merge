use serde::Deserialize;

use crate::file_listing::File;

#[derive(Deserialize, Debug, Clone)]
pub struct Mod {
    pub name: String,
    // pub summary: String,
    // pub description: String,
    pub mod_id: u32,
    pub domain_name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Md5LookupResult {
    #[serde(rename = "mod")]
    pub mod_details: Mod,
    pub file_details: File,
}
