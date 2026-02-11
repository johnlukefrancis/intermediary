// Path: crates/im_agent/src/protocol/events.rs
// Description: Agent event payloads and file entry types

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    Docs,
    Code,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileChangeType {
    Add,
    Change,
    Unlink,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub path: String,
    pub kind: FileKind,
    pub change_type: FileChangeType,
    pub mtime: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "StagedInfoWire")]
pub struct StagedInfo {
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
pub struct FileChangedEvent {
    pub repo_id: String,
    pub path: String,
    pub kind: FileKind,
    pub change_type: FileChangeType,
    pub mtime: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staged: Option<StagedInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotEvent {
    pub repo_id: String,
    pub recent: Vec<FileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "BundleBuiltEventWire")]
pub struct BundleBuiltEvent {
    pub repo_id: String,
    pub preset_id: String,
    pub host_path: String,
    #[serde(rename = "windowsPath", skip_serializing_if = "Option::is_none")]
    pub legacy_windows_path: Option<String>,
    pub alias_host_path: String,
    #[serde(rename = "aliasWindowsPath", skip_serializing_if = "Option::is_none")]
    pub legacy_alias_windows_path: Option<String>,
    pub bytes: u64,
    pub file_count: u64,
    pub built_at_iso: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StagedInfoWire {
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
struct BundleBuiltEventWire {
    repo_id: String,
    preset_id: String,
    #[serde(default)]
    host_path: Option<String>,
    #[serde(default)]
    windows_path: Option<String>,
    #[serde(default)]
    alias_host_path: Option<String>,
    #[serde(default)]
    alias_windows_path: Option<String>,
    bytes: u64,
    file_count: u64,
    built_at_iso: String,
}

impl TryFrom<StagedInfoWire> for StagedInfo {
    type Error = String;

    fn try_from(value: StagedInfoWire) -> Result<Self, Self::Error> {
        let (host_path, legacy_windows_path) = resolve_required_path_pair(
            value.host_path,
            value.windows_path,
            "hostPath",
            "windowsPath",
        )?;

        Ok(Self {
            host_path,
            legacy_windows_path,
            wsl_path: value.wsl_path,
            bytes_copied: value.bytes_copied,
            mtime_ms: value.mtime_ms,
        })
    }
}

impl TryFrom<BundleBuiltEventWire> for BundleBuiltEvent {
    type Error = String;

    fn try_from(value: BundleBuiltEventWire) -> Result<Self, Self::Error> {
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
            alias_host_path,
            legacy_alias_windows_path,
            bytes: value.bytes,
            file_count: value.file_count,
            built_at_iso: value.built_at_iso,
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
pub struct BundleBuildProgressEvent {
    pub repo_id: String,
    pub preset_id: String,
    pub phase: String,
    pub files_done: u64,
    pub files_total: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_bytes_done: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_bytes_total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_done_total_best_effort: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentErrorCode {
    WatcherInotifyLimit,
    WatcherFdLimit,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentEvent {
    #[serde(rename = "fileChanged")]
    FileChanged(FileChangedEvent),
    #[serde(rename = "snapshot")]
    Snapshot(SnapshotEvent),
    #[serde(rename = "bundleBuilt")]
    BundleBuilt(BundleBuiltEvent),
    #[serde(rename = "bundleBuildProgress")]
    BundleBuildProgress(BundleBuildProgressEvent),
    #[serde(rename = "error")]
    Error(AgentErrorEvent),
    #[serde(rename = "wslBackendStatus")]
    WslBackendStatus(WslBackendStatusEvent),
}

impl FileChangedEvent {
    pub fn new(
        repo_id: String,
        path: String,
        kind: FileKind,
        change_type: FileChangeType,
        mtime: String,
    ) -> Self {
        Self {
            repo_id,
            path,
            kind,
            change_type,
            mtime,
            staged: None,
        }
    }
}

impl SnapshotEvent {
    pub fn new(repo_id: String, recent: Vec<FileEntry>) -> Self {
        Self { repo_id, recent }
    }
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
