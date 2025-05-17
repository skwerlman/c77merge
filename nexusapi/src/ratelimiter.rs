use std::{
    fmt::Debug,
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::{DateTime, FixedOffset, Utc};
use reqwest::header::HeaderMap;
use tracing::{debug, trace, warn};

const HOURLY_LIMIT_DEFAULT: u32 = 100;
const DAILY_LIMIT_DEFAULT: u32 = 2500;
const HOURLY_REMAINING_DEFAULT: u32 = HOURLY_LIMIT_DEFAULT;
const DAILY_REMAINING_DEFAULT: u32 = DAILY_LIMIT_DEFAULT;

type RateLimiter = Arc<Box<Mutex<Limiter>>>;

#[derive(Debug)]
pub(crate) struct Limiter {
    hourly_limit: u32,
    daily_limit: u32,
    hourly_remaining: u32,
    daily_remaining: u32,
    hourly_reset: DateTime<Utc>,
    daily_reset: DateTime<Utc>,
}

impl Limiter {
    fn limit_from_header<T: FromStr + Clone>(headers: &HeaderMap, header: &str, default: T) -> T
    where
        <T as std::str::FromStr>::Err: std::fmt::Debug,
    {
        headers
            .get(header)
            .map(|hv| {
                hv.to_str()
                    .unwrap_or_else(|v| {
                        debug!("faulty header: {} -> {:?}", header, v);
                        debug!("replacing bad header value with empty string");
                        "" // return an empty str. this gets caught by parse
                    })
                    .parse()
                    .unwrap_or_else(|v| {
                        debug!("faulty header: {} -> {:?}", header, v);
                        warn!("invalid header: {}", header);
                        default.clone()
                    })
            })
            .unwrap_or_else(|| {
                debug!("missing header: {}", header);
                default
            })
    }

    pub fn new(headers: &HeaderMap) -> Self {
        let hourly_limit =
            Self::limit_from_header(headers, "x-rl-hourly-limit", HOURLY_LIMIT_DEFAULT);
        let daily_limit = Self::limit_from_header(headers, "x-rl-daily-limit", DAILY_LIMIT_DEFAULT);
        let hourly_remaining =
            Self::limit_from_header(headers, "x-rl-hourly-remaining", HOURLY_REMAINING_DEFAULT);
        let daily_remaining =
            Self::limit_from_header(headers, "x-rl-daily-remaining", DAILY_REMAINING_DEFAULT);
        let hourly_reset = Self::limit_from_header::<DateTime<FixedOffset>>(
            headers,
            "x-rl-hourly-reset",
            DateTime::<Utc>::MIN_UTC.into(),
        )
        .into();
        let daily_reset = Self::limit_from_header::<DateTime<FixedOffset>>(
            headers,
            "x-rl-daily-reset",
            DateTime::<Utc>::MIN_UTC.into(),
        )
        .into();

        let limiter = Self {
            hourly_limit,
            daily_limit,
            hourly_remaining,
            daily_remaining,
            hourly_reset,
            daily_reset,
        };

        debug!("initialized rate limiter");
        trace!("{:?}", limiter);

        limiter
    }

    pub fn refresh_limits(&mut self, headers: &HeaderMap) -> &Self {
        trace!("refreshing rate limiter limits...");

        self.hourly_limit =
            Self::limit_from_header(headers, "x-rl-hourly-limit", self.hourly_limit);
        self.daily_limit = Self::limit_from_header(headers, "x-rl-daily-limit", self.daily_limit);
        self.hourly_remaining =
            Self::limit_from_header(headers, "x-rl-hourly-remaining", self.hourly_remaining);
        self.daily_remaining =
            Self::limit_from_header(headers, "x-rl-daily-remaining", self.daily_remaining);
        self.hourly_reset =
            Self::limit_from_header(headers, "x-rl-hourly-reset", self.hourly_reset);
        self.daily_reset = Self::limit_from_header(headers, "x-rl-daily-reset", self.daily_reset);

        trace!("refreshed limits: {:?}", self);

        self
    }

    fn acquire(&mut self) -> (&Self, bool) {
        let is_available = if self.hourly_remaining > 0 && self.daily_remaining > 0 {
            self.hourly_remaining -= 1;
            self.daily_remaining -= 1;
            true
        } else {
            false
        };

        (self, is_available)
    }
}

#[derive(Debug, Clone)]
pub struct LimiterAdapter {
    limiter_mutex: RateLimiter,
}

impl LimiterAdapter {
    pub(crate) fn new(headers: &HeaderMap) -> Self {
        let limiter_mutex: RateLimiter = Arc::new(Box::new(Limiter::new(headers).into()));

        Self { limiter_mutex }
    }

    pub(crate) fn middleware(self) -> reqwest_ratelimit::Middleware<Self> {
        reqwest_ratelimit::all(self)
    }

    pub(crate) fn mutex_ref(&self) -> RateLimiter {
        self.limiter_mutex.clone()
    }

    pub(crate) fn refresh_limits(&self, headers: &HeaderMap) {
        self.mutex_ref().lock().unwrap().refresh_limits(headers);
    }
}

impl reqwest_ratelimit::RateLimiter for LimiterAdapter {
    async fn acquire_permit(&self) {
        // lock() polls the mutex until it gets access, so we iterate only once each time we get access to the limiter
        // we only do the sleep if the limiter has no tickets left
        while let (_, false) = self.limiter_mutex.lock().unwrap().acquire() {
            std::thread::sleep(Duration::from_millis(250));
        }
    }
}
