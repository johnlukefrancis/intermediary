// Path: crates/im_agent/src/protocol/responses.rs
// Description: Agent-to-UI response payloads for the WebSocket protocol

use serde::{Deserialize, Serialize};

use super::responses_legacy_wire::{BuildBundleResultWire, BundleInfoWire, StageFileResultWire};
use super::responses_repo::{GetRepoTopLevelResult, ListRepoDirectoryResult};
use super::responses_source_control::{
    SourceControlActionResult, SourceControlDiffResult, SourceControlStatusResult,
};
use super::responses_tr_fleet::{GetTrFleetStatusResult, TrFleetActionResult};

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
#[serde(rename_all = "camelCase", try_from = "StageFileResultWire")]
pub struct StageFileResult {
    pub repo_id: String,
    pub path: String,
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
pub struct ReadTextFileResult {
    pub repo_id: String,
    pub path: String,
    pub content: String,
    pub bytes: u64,
    pub mtime_ms: u64,
    pub encoding: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadImageFileResult {
    pub repo_id: String,
    pub path: String,
    pub data_base64: String,
    pub mime_type: String,
    pub bytes: u64,
    pub mtime_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "BuildBundleResultWire")]
pub struct BuildBundleResult {
    pub repo_id: String,
    pub preset_id: String,
    pub host_path: String,
    #[serde(rename = "windowsPath", skip_serializing_if = "Option::is_none")]
    pub legacy_windows_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wsl_path: Option<String>,
    pub alias_host_path: String,
    #[serde(rename = "aliasWindowsPath", skip_serializing_if = "Option::is_none")]
    pub legacy_alias_windows_path: Option<String>,
    pub bytes: u64,
    pub file_count: u64,
    pub built_at_iso: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelBundleBuildResult {
    pub repo_id: String,
    pub preset_id: String,
    pub build_id: String,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "BundleInfoWire")]
pub struct BundleInfo {
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

/// The answer to `shutdown`: whether every mutation this agent owned finished
/// inside the drain budget, and how many were still holding their worktree
/// lock when the budget expired.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShutdownResult {
    pub drained: bool,
    pub active_mutations: u32,
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
    #[serde(rename = "readTextFileResult")]
    ReadTextFileResult(ReadTextFileResult),
    #[serde(rename = "readImageFileResult")]
    ReadImageFileResult(ReadImageFileResult),
    #[serde(rename = "buildBundleResult")]
    BuildBundleResult(BuildBundleResult),
    #[serde(rename = "cancelBundleBuildResult")]
    CancelBundleBuildResult(CancelBundleBuildResult),
    #[serde(rename = "getRepoTopLevelResult")]
    GetRepoTopLevelResult(GetRepoTopLevelResult),
    #[serde(rename = "listRepoDirectoryResult")]
    ListRepoDirectoryResult(ListRepoDirectoryResult),
    #[serde(rename = "listBundlesResult")]
    ListBundlesResult(ListBundlesResult),
    #[serde(rename = "getTrFleetStatusResult")]
    GetTrFleetStatusResult(GetTrFleetStatusResult),
    #[serde(rename = "trFleetActionResult")]
    TrFleetActionResult(TrFleetActionResult),
    #[serde(rename = "sourceControlStatusResult")]
    SourceControlStatusResult(SourceControlStatusResult),
    #[serde(rename = "sourceControlDiffResult")]
    SourceControlDiffResult(SourceControlDiffResult),
    #[serde(rename = "sourceControlActionResult")]
    SourceControlActionResult(SourceControlActionResult),
    #[serde(rename = "shutdownResult")]
    ShutdownResult(ShutdownResult),
}
