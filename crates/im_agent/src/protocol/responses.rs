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
pub struct GetRepoTopLevelResult {
    pub repo_id: String,
    pub dirs: Vec<String>,
    pub files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subdirs: Option<std::collections::HashMap<String, Vec<String>>>,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StageFileResultWire {
    repo_id: String,
    path: String,
    #[serde(default)]
    host_path: Option<String>,
    #[serde(default)]
    windows_path: Option<String>,
    #[serde(default)]
    wsl_path: Option<String>,
    bytes_copied: u64,
    mtime_ms: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildBundleResultWire {
    repo_id: String,
    preset_id: String,
    #[serde(default)]
    host_path: Option<String>,
    #[serde(default)]
    windows_path: Option<String>,
    #[serde(default)]
    wsl_path: Option<String>,
    #[serde(default)]
    alias_host_path: Option<String>,
    #[serde(default)]
    alias_windows_path: Option<String>,
    bytes: u64,
    file_count: u64,
    built_at_iso: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundleInfoWire {
    #[serde(default)]
    host_path: Option<String>,
    #[serde(default)]
    windows_path: Option<String>,
    file_name: String,
    bytes: u64,
    mtime_ms: u64,
    is_latest_alias: bool,
}

impl TryFrom<StageFileResultWire> for StageFileResult {
    type Error = String;

    fn try_from(value: StageFileResultWire) -> Result<Self, Self::Error> {
        let (host_path, legacy_windows_path) = resolve_required_path_pair(
            value.host_path,
            value.windows_path,
            "hostPath",
            "windowsPath",
        )?;

        Ok(Self {
            repo_id: value.repo_id,
            path: value.path,
            host_path,
            legacy_windows_path,
            wsl_path: value.wsl_path,
            bytes_copied: value.bytes_copied,
            mtime_ms: value.mtime_ms,
        })
    }
}

impl TryFrom<BuildBundleResultWire> for BuildBundleResult {
    type Error = String;

    fn try_from(value: BuildBundleResultWire) -> Result<Self, Self::Error> {
        let (host_path, legacy_windows_path) = resolve_required_path_pair(
            value.host_path,
            value.windows_path,
            "hostPath",
            "windowsPath",
        )?;
        let (alias_host_path, legacy_alias_windows_path) = resolve_required_path_pair(
            value.alias_host_path,
            value.alias_windows_path,
            "aliasHostPath",
            "aliasWindowsPath",
        )?;

        Ok(Self {
            repo_id: value.repo_id,
            preset_id: value.preset_id,
            host_path,
            legacy_windows_path,
            wsl_path: value.wsl_path,
            alias_host_path,
            legacy_alias_windows_path,
            bytes: value.bytes,
            file_count: value.file_count,
            built_at_iso: value.built_at_iso,
        })
    }
}

impl TryFrom<BundleInfoWire> for BundleInfo {
    type Error = String;

    fn try_from(value: BundleInfoWire) -> Result<Self, Self::Error> {
        let (host_path, legacy_windows_path) = resolve_required_path_pair(
            value.host_path,
            value.windows_path,
            "hostPath",
            "windowsPath",
        )?;

        Ok(Self {
            host_path,
            legacy_windows_path,
            file_name: value.file_name,
            bytes: value.bytes,
            mtime_ms: value.mtime_ms,
            is_latest_alias: value.is_latest_alias,
        })
    }
}

fn resolve_required_path_pair(
    canonical: Option<String>,
    legacy: Option<String>,
    canonical_name: &str,
    legacy_name: &str,
) -> Result<(String, Option<String>), String> {
    match (canonical, legacy) {
        (Some(canonical), Some(legacy)) => {
            if canonical != legacy {
                return Err(format!("conflicting {canonical_name}/{legacy_name} values"));
            }
            Ok((canonical, Some(legacy)))
        }
        (Some(canonical), None) => Ok((canonical, None)),
        (None, Some(legacy)) => Ok((legacy.clone(), Some(legacy))),
        (None, None) => Err(format!("missing {canonical_name}")),
    }
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
