use reqwest::header::HeaderMap;
use serde::Deserialize;

use crate::error::Error;

#[derive(Debug)]
pub enum NexusValidateResult {
    ValidKey(NexusUser, HeaderMap),
    InvalidKey(Error),
}

#[derive(Debug, Deserialize)]
pub struct NexusUser {
    pub user_id: u32,
    pub name: String,
    pub is_premium: bool,
    pub is_supporter: bool,
    pub email: String,
    pub profile_url: String,
}
