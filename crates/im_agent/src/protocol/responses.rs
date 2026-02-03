// Path: crates/im_agent/src/protocol/responses.rs
// Description: Agent-to-UI response payloads for the WebSocket protocol

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientHelloResult {
    pub agent_version: String,
    pub watched_repo_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetOptionsResult {
    pub auto_stage_on_change: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UiResponse {
    #[serde(rename = "clientHelloResult")]
    ClientHelloResult(ClientHelloResult),
    #[serde(rename = "setOptionsResult")]
    SetOptionsResult(SetOptionsResult),
}
