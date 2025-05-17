use reqwest_middleware::ClientWithMiddleware;

#[derive(Debug, Clone)]
pub struct NexusClient<const RATE_LIMITED: bool, const VALID_DOMAIN: bool> {
    pub(super) client: ClientWithMiddleware,
    pub game_domain: String,
}

impl<'a, const RATE_LIMITED: bool, const VALID_DOMAIN: bool>
    NexusClient<RATE_LIMITED, VALID_DOMAIN>
{
    pub fn with_domain(self, domain: &'a str) -> NexusClient<RATE_LIMITED, true> {
        NexusClient {
            client: self.client,
            game_domain: domain.to_string(),
        }
    }

    pub fn ratelimited(&self) -> bool {
        RATE_LIMITED
    }

    pub fn valid(&self) -> bool {
        VALID_DOMAIN
    }
}

impl<const RATE_LIMITED: bool, const VALID_DOMAIN: bool>
    From<NexusClient<RATE_LIMITED, VALID_DOMAIN>> for ClientWithMiddleware
{
    fn from(value: NexusClient<RATE_LIMITED, VALID_DOMAIN>) -> Self {
        value.client
    }
}

impl<const RATE_LIMITED: bool, const VALID_DOMAIN: bool> From<ClientWithMiddleware>
    for NexusClient<RATE_LIMITED, VALID_DOMAIN>
{
    fn from(client: ClientWithMiddleware) -> Self {
        Self {
            client,
            game_domain: String::default(),
        }
    }
}
