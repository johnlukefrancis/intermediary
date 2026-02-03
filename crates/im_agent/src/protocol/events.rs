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
#[serde(rename_all = "camelCase")]
pub struct StagedInfo {
    pub wsl_path: String,
    pub windows_path: String,
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
#[serde(rename_all = "camelCase")]
pub struct BundleBuiltEvent {
    pub repo_id: String,
    pub preset_id: String,
    pub windows_path: String,
    pub alias_windows_path: String,
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
        Self {
            repo_id,
            recent,
        }
    }
}

impl AgentErrorEvent {
    pub fn new(scope: impl Into<String>, message: impl Into<String>, details: Option<AgentErrorDetails>) -> Self {
        Self {
            scope: scope.into(),
            message: message.into(),
            details,
        }
    }
}
