// Path: crates/im_agent/src/protocol/responses_tr_fleet.rs
// Description: TR fleet response payload types for host-agent build-server control

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrFleetEndpointErrorCode {
    Timeout,
    Unreachable,
    HttpError,
    InvalidJson,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrFleetEndpointError {
    pub code: TrFleetEndpointErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrFleetTargetStatus {
    pub port: u16,
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doctor: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_error: Option<TrFleetEndpointError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doctor_error: Option<TrFleetEndpointError>,
    pub fetched_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTrFleetStatusResult {
    pub targets: Vec<TrFleetTargetStatus>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TrFleetActionKind {
    Rebuild,
    RestartWatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrFleetActionResult {
    pub action: TrFleetActionKind,
    pub port: u16,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_body: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<TrFleetEndpointError>,
}
