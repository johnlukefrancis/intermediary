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
#[serde(rename_all = "camelCase")]
pub struct WatchRepoResult {
    pub repo_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshResult {
    pub repo_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageFileResult {
    pub repo_id: String,
    pub path: String,
    #[serde(alias = "windowsPath")]
    pub host_path: String,
    #[serde(rename = "windowsPath", skip_serializing_if = "Option::is_none")]
    pub legacy_windows_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wsl_path: Option<String>,
    pub bytes_copied: u64,
    pub mtime_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildBundleResult {
    pub repo_id: String,
    pub preset_id: String,
    #[serde(alias = "windowsPath")]
    pub host_path: String,
    #[serde(rename = "windowsPath", skip_serializing_if = "Option::is_none")]
    pub legacy_windows_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wsl_path: Option<String>,
    #[serde(alias = "aliasWindowsPath")]
    pub alias_host_path: String,
    #[serde(rename = "aliasWindowsPath", skip_serializing_if = "Option::is_none")]
    pub legacy_alias_windows_path: Option<String>,
    pub bytes: u64,
    pub file_count: u64,
    pub built_at_iso: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRepoTopLevelResult {
    pub repo_id: String,
    pub dirs: Vec<String>,
    pub files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subdirs: Option<std::collections::HashMap<String, Vec<String>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleInfo {
    #[serde(alias = "windowsPath")]
    pub host_path: String,
    #[serde(rename = "windowsPath", skip_serializing_if = "Option::is_none")]
    pub legacy_windows_path: Option<String>,
    pub file_name: String,
    pub bytes: u64,
    pub mtime_ms: u64,
    pub is_latest_alias: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListBundlesResult {
    pub repo_id: String,
    pub preset_id: String,
    pub bundles: Vec<BundleInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UiResponse {
    #[serde(rename = "clientHelloResult")]
    ClientHelloResult(ClientHelloResult),
    #[serde(rename = "setOptionsResult")]
    SetOptionsResult(SetOptionsResult),
    #[serde(rename = "watchRepoResult")]
    WatchRepoResult(WatchRepoResult),
    #[serde(rename = "refreshResult")]
    RefreshResult(RefreshResult),
    #[serde(rename = "stageFileResult")]
    StageFileResult(StageFileResult),
    #[serde(rename = "buildBundleResult")]
    BuildBundleResult(BuildBundleResult),
    #[serde(rename = "getRepoTopLevelResult")]
    GetRepoTopLevelResult(GetRepoTopLevelResult),
    #[serde(rename = "listBundlesResult")]
    ListBundlesResult(ListBundlesResult),
}
