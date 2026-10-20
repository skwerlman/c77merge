use http::{Extensions, HeaderValue};
use reqwest::{Request, Response};
use reqwest_middleware::{Middleware, Next};

use crate::Result;

#[derive(Debug, Clone)]
pub(crate) struct HeaderOverrideMiddleware {
    pub header: &'static str,
    pub value: HeaderValue,
}

impl HeaderOverrideMiddleware {
    pub fn new(header: &'static str, value: &'static str) -> Result<Self> {
        let value = HeaderValue::from_str(value)?;
        Ok(Self { header, value })
    }
}

#[async_trait::async_trait]
impl Middleware for HeaderOverrideMiddleware {
    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> reqwest_middleware::Result<Response> {
        let mut res: Response = next.run(req, extensions).await?;

        res.headers_mut().insert(self.header, self.value.clone());

        Ok(res)
    }
}
