// Path: src-tauri/src/lib/agent/types.rs
// Description: Types for supervising host + WSL agent lifecycles

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSupervisorConfig {
    pub port: u16,
    pub auto_start: bool,
    #[serde(default)]
    pub distro: Option<String>,
    #[serde(default)]
    pub requires_wsl: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSupervisorStatus {
    Started,
    AlreadyRunning,
    Disabled,
    Backoff,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSupervisorResult {
    pub status: AgentSupervisorStatus,
    pub port: u16,
    pub wsl_port: u16,
    pub requires_wsl: bool,
    pub agent_dir: String,
    pub log_dir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
