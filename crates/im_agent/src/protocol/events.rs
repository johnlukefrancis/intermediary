// Path: crates/im_agent/src/protocol/events.rs
// Description: Agent event payloads and file entry types

use serde::{Deserialize, Serialize};

use super::events_legacy_wire::{BundleBuiltEventWire, StagedInfoWire};
use super::events_runtime::{AgentErrorEvent, WslBackendStatusEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    Docs,
    Code,
    Image,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileChangeType {
    Add,
    Change,
    Unlink,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileActivityBucket {
    pub bucket_start_iso: String,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileActivity {
    pub first_seen_at_iso: String,
    pub last_seen_at_iso: String,
    pub update_count: u32,
    pub burst_count: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<FileActivityBucket>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity: Option<FileActivity>,
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
    pub activity: Option<FileActivity>,
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
#[serde(rename_all = "camelCase")]
pub struct RepoTopologyChangedEvent {
    pub repo_id: String,
}

impl RepoTopologyChangedEvent {
    pub fn new(repo_id: String) -> Self {
        Self { repo_id }
    }
}

/// The repository's Git state or working tree changed in a way that can move
/// `git status`; coalesced by the watcher so bursts arrive as one event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceControlChangedEvent {
    pub repo_id: String,
}

impl SourceControlChangedEvent {
    pub fn new(repo_id: String) -> Self {
        Self { repo_id }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentEvent {
    #[serde(rename = "fileChanged")]
    FileChanged(FileChangedEvent),
    #[serde(rename = "snapshot")]
    Snapshot(SnapshotEvent),
    #[serde(rename = "repoTopologyChanged")]
    RepoTopologyChanged(RepoTopologyChangedEvent),
    #[serde(rename = "bundleBuilt")]
    BundleBuilt(BundleBuiltEvent),
    #[serde(rename = "bundleBuildProgress")]
    BundleBuildProgress(BundleBuildProgressEvent),
    #[serde(rename = "error")]
    Error(AgentErrorEvent),
    #[serde(rename = "wslBackendStatus")]
    WslBackendStatus(WslBackendStatusEvent),
    #[serde(rename = "sourceControlChanged")]
    SourceControlChanged(SourceControlChangedEvent),
}

impl FileChangedEvent {
    pub fn new(
        repo_id: String,
        path: String,
        kind: FileKind,
        change_type: FileChangeType,
        mtime: String,
        activity: Option<FileActivity>,
    ) -> Self {
        Self {
            repo_id,
            path,
            kind,
            change_type,
            mtime,
            activity,
            staged: None,
        }
    }
}

impl SnapshotEvent {
    pub fn new(repo_id: String, recent: Vec<FileEntry>) -> Self {
        Self { repo_id, recent }
    }
}
