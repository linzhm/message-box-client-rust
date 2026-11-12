//! Typed MessageBox wire client. See README for authentication interoperability limits.
//! No message encryption, payment processing, auto-acknowledgement or retries here.
pub mod transport;
pub mod types;
pub use bsv;
use serde::{de::DeserializeOwned, Serialize};
use transport::{Request, Transport};
pub use types::*;
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("authentication transport failed: {0}")]
    Authentication(String),
    #[error("request timed out; a write may already have completed")]
    Timeout,
    #[error("HTTP {status}")]
    Http {
        status: u16,
        error: Option<ServerError>,
        body: Vec<u8>,
    },
    #[error("JSON response or request is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("response exceeds client decoding limit")]
    ResponseTooLarge,
    #[error("unexpected response: {0}")]
    Protocol(String),
}
/// A caller-selected origin; no production endpoint is implicit.
pub struct Client<T> {
    origin: Url,
    transport: T,
    max_response_bytes: usize,
}
impl<T: Transport> Client<T> {
    pub fn new(origin: &str, transport: T) -> Result<Self, Error> {
        let url = Url::parse(origin).map_err(|e| Error::InvalidInput(e.to_string()))?;
        let local = match url.host() {
            Some(url::Host::Domain("localhost")) => true,
            Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
            Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
            _ => false,
        };
        if !(url.scheme() == "https" || url.scheme() == "http" && local)
            || url.host().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() != "/"
        {
            return Err(invalid("origin must be HTTPS (or loopback HTTP), without credentials, path, query or fragment"));
        }
        Ok(Self {
            origin: url,
            transport,
            max_response_bytes: 8 * 1024 * 1024,
        })
    }
    /// Limit decoding after transport buffering; the SDK does not expose a streaming cap.
    pub fn with_response_limit(mut self, bytes: usize) -> Self {
        self.max_response_bytes = bytes;
        self
    }
    async fn call<R: DeserializeOwned>(
        &self,
        method: &'static str,
        path: &str,
        query: Vec<(&str, String)>,
        body: Option<Vec<u8>>,
    ) -> Result<R, Error> {
        let mut url = self
            .origin
            .join(path)
            .map_err(|e| invalid(&e.to_string()))?;
        if !query.is_empty() {
            url.query_pairs_mut().extend_pairs(query);
        }
        let response = self
            .transport
            .execute(Request { url, method, body })
            .await?;
        if response.body.len() > self.max_response_bytes {
            return Err(Error::ResponseTooLarge);
        }
        if !(200..300).contains(&response.status) {
            return Err(Error::Http {
                status: response.status,
                error: serde_json::from_slice(&response.body).ok(),
                body: response.body,
            });
        }
        Ok(serde_json::from_slice(&response.body)?)
    }
    async fn post<B: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<R, Error> {
        self.call("POST", path, vec![], Some(serde_json::to_vec(body)?))
            .await
    }
    pub async fn send(&self, request: &SendRequest) -> Result<SendResponse, Error> {
        let m = &request.message;
        key(&m.recipient)?;
        exact(&m.message_box, 128)?;
        exact(&m.message_id, 256)?;
        if m.body.len() > 4 * 1024 * 1024 {
            return Err(invalid("body exceeds 4 MiB"));
        }
        let response: SendResponse = self.post("/sendMessage", request).await?;
        if response.results.len() != 1
            || !response.results[0]
                .recipient
                .eq_ignore_ascii_case(&m.recipient)
            || response.results[0].message_id != m.message_id
        {
            return Err(Error::Protocol(
                "send result does not match recipient and message ID".into(),
            ));
        }
        Ok(response)
    }
    pub async fn list(&self, request: &ListRequest) -> Result<ListResponse, Error> {
        exact(&request.message_box, 128)?;
        pagination(request.limit, request.offset, 1000)?;
        if let Some(id) = &request.message_id {
            exact(id, 256)?;
        }
        let response: ListResponse = self.post("/listMessages", request).await?;
        if response.limit == 0
            || response.limit > request.limit.unwrap_or(1000)
            || response.offset != request.offset.unwrap_or(0)
            || response.messages.len() > response.limit as usize
            || response.next_offset != response.offset + response.messages.len() as u32
            || response.has_more && response.messages.is_empty()
        {
            return Err(Error::Protocol("invalid list pagination".into()));
        }
        Ok(response)
    }
    pub async fn acknowledge(&self, request: &AckRequest) -> Result<AckResponse, Error> {
        if request.message_ids.is_empty() || request.message_ids.len() > 1000 {
            return Err(invalid("ack requires 1..=1000 IDs"));
        }
        for id in &request.message_ids {
            exact(id, 256)?;
        }
        self.post("/acknowledgeMessage", request).await
    }
    pub async fn register_device(
        &self,
        request: &RegisterDeviceRequest,
    ) -> Result<RegisterDeviceResponse, Error> {
        if request.fcm_token.trim().is_empty() || request.fcm_token.len() > 500 {
            return Err(invalid("invalid device token length"));
        }
        if request.device_id.as_ref().is_some_and(|id| id.len() > 255) {
            return Err(invalid("device ID exceeds 255 bytes"));
        }
        self.post("/registerDevice", request).await
    }
    pub async fn devices(&self, page: &Pagination) -> Result<DevicesResponse, Error> {
        self.call(
            "GET",
            "/devices",
            pagination(page.limit, page.offset, 100)?,
            None,
        )
        .await
    }
    pub async fn set_permission(
        &self,
        request: &SetPermissionRequest,
    ) -> Result<SetPermissionResponse, Error> {
        exact(&request.message_box, 128)?;
        if let Some(sender) = &request.sender {
            key(sender)?;
        }
        if request.recipient_fee < -1 {
            return Err(invalid("recipientFee must be -1..=2147483647"));
        }
        self.post("/permissions/set", request).await
    }
    pub async fn permission(
        &self,
        message_box: &str,
        sender: Option<&str>,
    ) -> Result<PermissionResponse, Error> {
        exact(message_box, 128)?;
        let mut q = vec![("messageBox", message_box.to_owned())];
        if let Some(sender) = sender {
            key(sender)?;
            q.push(("sender", sender.to_owned()));
        }
        self.call("GET", "/permissions/get", q, None).await
    }
    pub async fn permissions(
        &self,
        request: &ListPermissionsRequest,
    ) -> Result<PermissionsResponse, Error> {
        let mut q = pagination(request.page.limit, request.page.offset, 100)?;
        if let Some(name) = &request.message_box {
            exact(name, 128)?;
            q.push(("messageBox", name.clone()));
        }
        if let Some(order) = request.created_at_order {
            q.push((
                "createdAtOrder",
                match order {
                    SortOrder::Asc => "asc",
                    SortOrder::Desc => "desc",
                }
                .into(),
            ));
        }
        self.call("GET", "/permissions/list", q, None).await
    }
    /// Read-only single-recipient quote; never authorizes a payment.
    pub async fn quote(&self, recipient: &str, message_box: &str) -> Result<QuoteResponse, Error> {
        key(recipient)?;
        exact(message_box, 128)?;
        let response: QuoteResponse = self
            .call(
                "GET",
                "/permissions/quote",
                vec![
                    ("recipient", recipient.to_owned()),
                    ("messageBox", message_box.to_owned()),
                ],
                None,
            )
            .await?;
        if response.quote.recipient_fee < -1 || response.quote.delivery_fee > i32::MAX as u32 {
            return Err(Error::Protocol("invalid quote fee".into()));
        }
        Ok(response)
    }
}
fn invalid(message: &str) -> Error {
    Error::InvalidInput(message.into())
}
fn exact(s: &str, max: usize) -> Result<(), Error> {
    if s.is_empty() || s.len() > max || s.trim() != s || s.chars().any(char::is_control) {
        return Err(invalid(
            "text must be exact, bounded, nonempty and control-free",
        ));
    }
    Ok(())
}
fn key(s: &str) -> Result<(), Error> {
    if s.len() != 66
        || !(s.starts_with("02") || s.starts_with("03"))
        || !s.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(invalid("expected compressed public key hex"));
    }
    bsv::primitives::public_key::PublicKey::from_string(s)
        .map_err(|_| invalid("invalid public key"))?;
    Ok(())
}
fn pagination(
    limit: Option<u32>,
    offset: Option<u32>,
    max: u32,
) -> Result<Vec<(&'static str, String)>, Error> {
    if limit.is_some_and(|n| n == 0 || n > max) || offset.is_some_and(|n| n > 100_000) {
        return Err(invalid("pagination out of bounds"));
    }
    let mut q = vec![];
    if let Some(n) = limit {
        q.push(("limit", n.to_string()));
    }
    if let Some(n) = offset {
        q.push(("offset", n.to_string()));
    }
    Ok(q)
}
