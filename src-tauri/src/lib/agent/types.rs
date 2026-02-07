// Path: src-tauri/src/lib/agent/types.rs
// Description: Types for supervising host agent lifecycle with optional Windows WSL backend

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSupervisorWslConfig {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub distro: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSupervisorConfig {
    pub port: u16,
    pub auto_start: bool,
    #[serde(default)]
    pub wsl: Option<AgentSupervisorWslConfig>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSupervisorStatus {
    Started,
    AlreadyRunning,
    Disabled,
    Backoff,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSupervisorWslStatus {
    pub required: bool,
    pub port: u16,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSupervisorResult {
    pub status: AgentSupervisorStatus,
    pub port: u16,
    pub supports_wsl: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wsl: Option<AgentSupervisorWslStatus>,
    pub agent_dir: String,
    pub log_dir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
