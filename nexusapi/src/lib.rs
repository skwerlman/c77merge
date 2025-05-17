use std::{sync::Arc, time::Duration};

use http_cache_reqwest::{
    CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions, MokaManager,
};
use moka::future::Cache as MokaCache;
use reqwest::{Client, StatusCode, header::HeaderMap};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{Jitter, RetryTransientMiddleware, policies::ExponentialBackoff};
use tracing::{error, info, instrument, trace};

pub(crate) mod client;
pub(crate) mod middleware;
pub(crate) mod ratelimiter;

pub mod error;
pub mod file_listing;
pub mod md5lookup;
pub mod user;

pub type Result<T> = std::result::Result<T, error::Error>;

use c77conf::Configuration;
use client::NexusClient;
use file_listing::FileListing;
use md5lookup::Md5LookupResult;
use middleware::HeaderOverrideMiddleware;
use ratelimiter::LimiterAdapter;
use user::{NexusUser, NexusValidateResult};

// see build.rs
include!(concat!(env!("OUT_DIR"), "/user_agent.rs"));

const API_BASE: &str = "https://api.nexusmods.com/";

fn add_middleware(config: &Configuration, client: ClientBuilder) -> Result<ClientBuilder> {
    let retry_policy = ExponentialBackoff::builder()
        .retry_bounds(Duration::from_millis(100), Duration::from_millis(15000))
        .jitter(Jitter::Bounded)
        .base(2)
        .build_with_total_retry_duration(Duration::from_secs(30));

    let moka_cache = MokaCache::new(5_000);
    let moka_manager = MokaManager {
        cache: Arc::new(moka_cache),
    };

    let memory_cache = Cache(HttpCache {
        mode: CacheMode::Default,
        manager: moka_manager,
        options: HttpCacheOptions::default(),
    });

    let disk_cache = Cache(HttpCache {
        mode: CacheMode::Default,
        manager: CACacheManager {
            path: config.nexus.cache_path.clone(),
        },
        options: HttpCacheOptions::default(),
    });

    // override cache-control header to enable successful api responses to be cached for up to an hour
    let header_override = HeaderOverrideMiddleware::new("cache-control", "max-age=14400")?;

    Ok(client
        .with(header_override.clone())
        .with(memory_cache)
        .with(header_override.clone())
        .with(disk_cache)
        .with(header_override.clone())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .with(header_override.clone()))
}

fn inner_client(config: &Configuration) -> Result<Client> {
    let conf = &config.nexus;
    let mut headers = HeaderMap::new();
    headers.insert("apikey", conf.api_key.parse()?);
    headers.insert("accept", "application/json".parse()?);

    let inner_client = Client::builder()
        // from user_agent.rs
        .user_agent(USER_AGENT)
        .https_only(conf.forbid_http)
        .timeout(Duration::from_millis(conf.timeout))
        .connect_timeout(Duration::from_millis(conf.connect_timeout))
        .read_timeout(Duration::from_millis(conf.read_timeout))
        .connection_verbose(conf.verbose_connection)
        .brotli(conf.enable_compression)
        .deflate(conf.enable_compression)
        .gzip(conf.enable_compression)
        .zstd(conf.enable_compression)
        .default_headers(headers)
        .build()?;

    Ok(inner_client)
}

#[instrument(level = "trace", skip(config))]
pub async fn login(
    config: Arc<Configuration>,
) -> Result<(NexusClient<true, false>, NexusUser, LimiterAdapter)> {
    let cfg_copy = config.clone();

    info!("Logging in...");

    let api_client_no_ratelimit: NexusClient<false, false> =
        add_middleware(&cfg_copy, ClientBuilder::new(inner_client(&cfg_copy)?))?
            .build()
            .into();

    let res = validate(api_client_no_ratelimit.clone()).await?;

    let api_client: NexusClient<true, false>;
    let userinfo: NexusUser;
    let limiter: LimiterAdapter;

    match res {
        NexusValidateResult::ValidKey(info, headers) => {
            limiter = LimiterAdapter::new(&headers);
            api_client = ClientBuilder::from_client(api_client_no_ratelimit.client)
                .with(limiter.clone().middleware())
                .build()
                .into();
            userinfo = info;
        }
        NexusValidateResult::InvalidKey(error) => {
            error!("{}", error);
            return Err(error);
        }
    };

    info!("Logged in to nexus as {}", userinfo.name);
    trace!("userinfo: {:?}", userinfo);

    Ok((api_client, userinfo, limiter))
}

#[instrument(level = "trace", skip(client))]
async fn validate<const VALID_DOMAIN: bool>(
    client: NexusClient<false, VALID_DOMAIN>,
) -> Result<NexusValidateResult> {
    let mut route = API_BASE.to_owned();
    route.push_str("v1/users/validate.json");

    let c: ClientWithMiddleware = client.into();
    let resp = c.get(route).send().await?;

    if resp.status().is_success() {
        let headers = resp.headers().clone();
        Ok(NexusValidateResult::ValidKey(resp.json().await?, headers))
    } else if resp.status().is_client_error() {
        Ok(NexusValidateResult::InvalidKey(resp.json().await?))
    } else {
        Err(format!("unexpected response from api: {:?}", resp).into())
    }
}

#[instrument(level = "trace", skip_all)]
pub async fn refresh_limits<const VALID_DOMAIN: bool>(
    client: NexusClient<true, VALID_DOMAIN>,
    limiter: &LimiterAdapter,
) -> Result<()> {
    info!("Refreshing rate limits...");
    let c: ClientWithMiddleware = client.into();

    let res = c.head(API_BASE).send().await?;

    limiter.refresh_limits(res.headers());

    info!("Limits refreshed");

    Ok(())
}

#[instrument(level = "trace", skip(client))]
pub async fn get_by_hash(client: NexusClient<true, true>, hash: String) -> Result<Md5LookupResult> {
    let mut route = API_BASE.to_owned();
    route.push_str("v1/games/");
    route.push_str(&client.game_domain);
    route.push_str("/mods/md5_search/");
    route.push_str(hash.as_str());
    route.push_str(".json");

    let c: ClientWithMiddleware = client.into();

    let resp = c.get(route).send().await?;

    if resp.status() == StatusCode::NOT_FOUND {
        return Err("Mod not found".into());
    } else if resp.status() == StatusCode::UNPROCESSABLE_ENTITY {
        return Err("Invalid hash".into());
    } else if !resp.status().is_success() {
        return Err(format!("Unexpected status code: {} ", resp.status()).into());
    }

    let lookups: Vec<Md5LookupResult> = serde_json::from_str(&resp.text().await?)?;

    trace!("lookup results: {:?}", lookups);

    let mut lookup: Option<Md5LookupResult> = None;

    for l in lookups {
        if let Some(ref newest) = lookup {
            if newest.file_details.file_id < l.file_details.file_id {
                trace!("taking newer: {:?}", l);
                lookup = Some(l);
            } else {
                trace!("keeping: {:?}", newest)
            }
        } else {
            trace!("starting with: {:?}", l);
            lookup = Some(l);
        }
    }

    lookup.ok_or("Something went wrong finding the maximum file id!".into())
}

#[instrument(level = "trace", skip(client))]
pub async fn get_file_listing(
    client: NexusClient<true, true>,
    file_id: u64,
) -> Result<FileListing> {
    let mut route = API_BASE.to_owned();
    route.push_str("v1/games/");
    route.push_str(&client.game_domain);
    route.push_str("/mods/");
    route.push_str(&file_id.to_string());
    route.push_str("/files.json");

    let c: ClientWithMiddleware = client.into();
    let resp = c.get(route).send().await?;

    if resp.status() == StatusCode::NOT_FOUND {
        return Err("Mod not found".into());
    } else if !resp.status().is_success() {
        return Err(format!("Unexpected status code: {} ", resp.status()).into());
    }

    let listing = serde_json::from_str(&resp.text().await?)?;

    Ok(listing)
}
