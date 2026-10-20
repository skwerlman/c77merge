use std::{sync::Arc, time::Duration};

use cacache::RemoveOpts;
use http_cache_reqwest::{
    CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions, MokaManager,
};
use moka::future::Cache as MokaCache;
use reqwest_middleware::ClientBuilder;

use reqwest_retry::{Jitter, RetryTransientMiddleware, policies::ExponentialBackoff};

use crate::{ClientConfig, Result};

pub(crate) mod header_override;
pub(crate) use header_override::HeaderOverrideMiddleware;

pub(crate) fn add_middleware(
    config: &impl ClientConfig,
    client: ClientBuilder,
) -> Result<ClientBuilder> {
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
            path: config.cache_path().to_owned(),
            remove_opts: RemoveOpts::default(),
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
