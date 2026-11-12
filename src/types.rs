//! Wire types for ts-stack 52d36a5 message-box-http.yaml and its route handlers.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Success {
    Success,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub recipient: String,
    pub message_box: String,
    pub message_id: String,
    /// Already serialized wire body; no implicit encryption or JSON rewriting.
    pub body: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct SendRequest {
    pub message: Message,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SentMessage {
    pub recipient: String,
    pub message_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendResponse {
    pub status: Success,
    pub message: String,
    pub results: Vec<SentMessage>,
}
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ListRequest {
    pub message_box: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    /// Supported by the pinned handler/client, although absent in the OpenAPI schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredMessage {
    pub message_id: String,
    pub body: String,
    pub sender: String,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResponse {
    pub status: Success,
    pub messages: Vec<StoredMessage>,
    pub limit: u32,
    pub offset: u32,
    pub next_offset: u32,
    pub has_more: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AckRequest {
    pub message_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckResponse {
    pub status: Success,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Ios,
    Android,
    Web,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterDeviceRequest {
    pub fcm_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<Platform>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterDeviceResponse {
    pub status: Success,
    pub message: String,
    pub device_id: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredDevice {
    pub id: u64,
    pub device_id: Option<String>,
    pub platform: Option<Platform>,
    pub fcm_token: String,
    #[serde(deserialize_with = "deserialize_active")]
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub last_used: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevicesResponse {
    pub status: Success,
    pub limit: u32,
    pub offset: u32,
    pub devices: Vec<RegisteredDevice>,
}
#[derive(Debug, Clone, Default)]
pub struct Pagination {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionStatus {
    AlwaysAllow,
    Blocked,
    PaymentRequired,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPermissionRequest {
    pub message_box: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<String>,
    pub recipient_fee: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Permission {
    pub sender: Option<String>,
    pub message_box: String,
    pub recipient_fee: i32,
    // listPermissions omits status; getPermission includes it.
    pub status: Option<PermissionStatus>,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetPermissionResponse {
    pub status: Success,
    pub description: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionResponse {
    pub status: Success,
    pub description: String,
    pub permission: Option<Permission>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionsResponse {
    pub status: Success,
    pub permissions: Vec<Permission>,
    pub total_count: u64,
}
#[derive(Debug, Clone, Copy)]
pub enum SortOrder {
    Asc,
    Desc,
}
#[derive(Debug, Clone, Default)]
pub struct ListPermissionsRequest {
    pub message_box: Option<String>,
    pub page: Pagination,
    pub created_at_order: Option<SortOrder>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Quote {
    pub delivery_fee: u32,
    pub recipient_fee: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteResponse {
    pub status: Success,
    pub description: String,
    pub quote: Quote,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerError {
    pub status: String,
    pub code: String,
    pub description: String,
    #[serde(flatten)]
    pub details: BTreeMap<String, serde_json::Value>,
}

// The pinned route forwards SQL booleans verbatim (MySQL/SQLite can return 0/1).
fn deserialize_active<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Active {
        Bool(bool),
        Integer(u8),
    }
    match Active::deserialize(d)? {
        Active::Bool(v) => Ok(v),
        Active::Integer(0) => Ok(false),
        Active::Integer(1) => Ok(true),
        _ => Err(serde::de::Error::custom("active must be boolean or 0/1")),
    }
}
