use std::marker::ConstParamTy;
use std::sync::Arc;
use std::time::Duration;

use http::HeaderMap;
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use tracing::{error, info, instrument, trace};

use crate::middleware::add_middleware;
use crate::ratelimiter::LimiterAdapter;
use crate::user::{NexusUser, NexusValidateResult};
use crate::{API_BASE, ClientConfig, Result};

// see build.rs
include!(concat!(env!("OUT_DIR"), "/user_agent.rs"));

#[derive(ConstParamTy, PartialEq, Eq)]
pub enum RateLimit {
    Yes,
    No,
}
pub const RATELIMITED: RateLimit = RateLimit::Yes;
pub const NOT_RATELIMITED: RateLimit = RateLimit::No;

#[derive(ConstParamTy, PartialEq, Eq)]
pub enum Domain {
    Valid,
    Invalid,
}
pub const VALIDDOMAIN: Domain = Domain::Valid;
pub const INVALIDDOMAIN: Domain = Domain::Invalid;

#[derive(Debug, Clone)]
pub struct NexusClient<const RATE_LIMITED: RateLimit, const VALID_DOMAIN: Domain> {
    pub(super) client: ClientWithMiddleware,
    pub game_domain: String,
}

impl<'a, const RATE_LIMITED: RateLimit, const VALID_DOMAIN: Domain>
    NexusClient<RATE_LIMITED, VALID_DOMAIN>
{
    pub async fn new(
        config: Arc<impl ClientConfig>,
    ) -> Result<(
        NexusClient<RATELIMITED, INVALIDDOMAIN>,
        NexusUser,
        LimiterAdapter,
    )> {
        let cfg_copy = config.clone();

        info!("Logging in...");

        let api_client_no_ratelimit: NexusClient<NOT_RATELIMITED, INVALIDDOMAIN> =
            add_middleware(&cfg_copy, ClientBuilder::new(inner_client(&cfg_copy)?))?
                .build()
                .into();

        let res = api_client_no_ratelimit.clone().validate().await?;

        let api_client: NexusClient<RATELIMITED, INVALIDDOMAIN>;
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
                error!("{error}");
                return Err(error);
            }
        };

        info!("Logged in to nexus as {}", userinfo.name);
        trace!("userinfo: {userinfo:?}");

        Ok((api_client, userinfo, limiter))
    }

    pub async fn refresh_limits(self, limiter: &LimiterAdapter) -> Result<()> {
        info!("Refreshing rate limits...");
        let c: ClientWithMiddleware = self.into();

        let res = c.head(API_BASE).send().await?;

        limiter.refresh_limits(res.headers());

        info!("Limits refreshed");

        Ok(())
    }

    #[instrument(level = "trace", skip(self))]
    async fn validate(self) -> Result<NexusValidateResult> {
        assert!(
            RATE_LIMITED == RateLimit::No,
            "Expected a client without a rate limiter installed"
        );
        let mut route = API_BASE.to_owned();
        route.push_str("v1/users/validate.json");

        let c: ClientWithMiddleware = self.into();
        let resp = c.get(route).send().await?;

        if resp.status().is_success() {
            let headers = resp.headers().clone();
            Ok(NexusValidateResult::ValidKey(resp.json().await?, headers))
        } else if resp.status().is_client_error() {
            Ok(NexusValidateResult::InvalidKey(resp.json().await?))
        } else {
            Err(format!("unexpected response from api: {resp:?}").into())
        }
    }

    pub fn with_domain(self, domain: &'a str) -> NexusClient<RATE_LIMITED, VALIDDOMAIN> {
        NexusClient {
            client: self.client,
            game_domain: domain.to_string(),
        }
    }

    #[allow(clippy::unused_self)]
    pub fn ratelimited(&self) -> RateLimit {
        RATE_LIMITED
    }

    #[allow(clippy::unused_self)]
    pub fn valid(&self) -> Domain {
        VALID_DOMAIN
    }
}

impl<const RATE_LIMITED: RateLimit, const VALID_DOMAIN: Domain>
    From<NexusClient<RATE_LIMITED, VALID_DOMAIN>> for ClientWithMiddleware
{
    fn from(value: NexusClient<RATE_LIMITED, VALID_DOMAIN>) -> Self {
        value.client
    }
}

impl<const RATE_LIMITED: RateLimit, const VALID_DOMAIN: Domain> From<ClientWithMiddleware>
    for NexusClient<RATE_LIMITED, VALID_DOMAIN>
{
    fn from(client: ClientWithMiddleware) -> Self {
        Self {
            client,
            game_domain: String::default(),
        }
    }
}

fn inner_client(config: &impl ClientConfig) -> Result<Client> {
    let mut headers = HeaderMap::new();
    headers.insert("apikey", config.api_key().parse()?);
    headers.insert("accept", "application/json".parse()?);

    let inner_client = Client::builder()
        // from user_agent.rs
        .user_agent(USER_AGENT)
        .https_only(config.forbid_http())
        .timeout(Duration::from_millis(config.timeout()))
        .connect_timeout(Duration::from_millis(config.connect_timeout()))
        .read_timeout(Duration::from_millis(config.read_timeout()))
        .connection_verbose(config.verbose_connection())
        .brotli(config.enable_compression())
        .deflate(config.enable_compression())
        .gzip(config.enable_compression())
        .zstd(config.enable_compression())
        .default_headers(headers)
        .build()?;

    Ok(inner_client)
}

#[instrument(level = "trace", skip(config))]
pub async fn login(
    config: Arc<impl ClientConfig>,
) -> Result<(
    NexusClient<RATELIMITED, INVALIDDOMAIN>,
    NexusUser,
    LimiterAdapter,
)> {
    NexusClient::<RATELIMITED, INVALIDDOMAIN>::new(config).await
}
