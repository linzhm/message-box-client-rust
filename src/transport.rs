//! Injectable transport. Implementations must authenticate before returning a response.
use crate::Error;
use async_trait::async_trait;
#[cfg(feature = "experimental-auth")]
use bsv::auth::clients::auth_fetch::{AuthFetch, FetchOptions};
#[cfg(feature = "experimental-auth")]
use bsv::wallet::proto_wallet::ProtoWallet;
#[cfg(feature = "experimental-auth")]
use std::{collections::HashMap, sync::Arc, time::Duration};

#[derive(Debug, Clone)]
pub struct Request {
    pub url: url::Url,
    pub method: &'static str,
    pub body: Option<Vec<u8>>,
}
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}
#[async_trait]
pub trait Transport: Send + Sync {
    async fn execute(&self, request: Request) -> Result<Response, Error>;
}
/// EXPERIMENTAL: SDK redirects are not constrained and no server identity pin is enforced.
/// Do not use with production secrets or untrusted endpoints.
/// BRC-103/104 authentication delegated to the pinned official SDK.
/// Only ProtoWallet is accepted: its create_action is unimplemented, so a 402
/// cannot create, sign or broadcast a transaction. A 402 may surface as Auth error.
#[cfg(feature = "experimental-auth")]
pub struct AuthTransport {
    auth: AuthFetch<Arc<ProtoWallet>>,
    timeout: Duration,
}
#[cfg(feature = "experimental-auth")]
impl AuthTransport {
    pub fn new(wallet: ProtoWallet) -> Self {
        Self {
            auth: AuthFetch::new(Arc::new(wallet)),
            timeout: Duration::from_secs(30),
        }
    }
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}
#[cfg(feature = "experimental-auth")]
#[async_trait]
impl Transport for AuthTransport {
    async fn execute(&self, request: Request) -> Result<Response, Error> {
        let mut headers = HashMap::new();
        if request.body.is_some() {
            headers.insert("content-type".to_owned(), "application/json".to_owned());
        }
        let response = tokio::time::timeout(
            self.timeout,
            self.auth.fetch_with_options(
                request.url.as_str(),
                request.method,
                request.body,
                Some(headers),
                FetchOptions {
                    payment_retry_attempts: Some(1),
                },
            ),
        )
        .await
        .map_err(|_| Error::Timeout)?
        .map_err(|e| Error::Authentication(e.to_string()))?;
        Ok(Response {
            status: response.status,
            body: response.body,
        })
    }
}
