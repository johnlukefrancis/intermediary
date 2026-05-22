// Path: crates/im_agent/src/protocol/events_runtime.rs
// Description: Runtime status and error event payloads

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentErrorCode {
    WatcherInotifyLimit,
    WatcherFdLimit,
    WatcherMountedWindowsPathRisk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentErrorDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<AgentErrorCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentErrorEvent {
    pub scope: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<AgentErrorDetails>,
}

impl AgentErrorEvent {
    pub fn new(
        scope: impl Into<String>,
        message: impl Into<String>,
        details: Option<AgentErrorDetails>,
    ) -> Self {
        Self {
            scope: scope.into(),
            message: message.into(),
            details,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WslBackendConnectionStatus {
    Online,
    Offline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WslBackendStatusEvent {
    pub status: WslBackendConnectionStatus,
    pub generation: u64,
}
