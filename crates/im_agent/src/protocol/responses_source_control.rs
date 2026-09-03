// Path: crates/im_agent/src/protocol/responses_source_control.rs
// Description: Agent-to-UI source-control payloads: working-tree status, per-file diff, action outcome

use serde::{Deserialize, Serialize};

use super::commands_source_control::{SourceControlActionKind, SourceControlArea};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceControlEntryArea {
    Index,
    Worktree,
    Conflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceControlChange {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Untracked,
    Unmerged,
}

/// One changed path in one area. A path modified in both the index and the
/// working tree appears once per area.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceControlEntry {
    /// Repo-root-relative slash path (same contract as `readTextFile`).
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_path: Option<String>,
    pub area: SourceControlEntryArea,
    pub change: SourceControlChange,
}

/// Changed paths that are counted but not listed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceControlOmitted {
    /// Staged paths outside the configured root (a subdirectory of the Git top
    /// level); a commit of the whole index carries them too. Worktree-only,
    /// untracked, and unmerged paths outside the root are not counted.
    pub staged_outside_root: u64,
    /// Paths whose bytes are not valid UTF-8 and cannot cross the wire losslessly.
    pub unrepresentable_path: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceControlStatus {
    pub branch: Option<String>,
    pub head_sha: Option<String>,
    pub detached: bool,
    pub upstream: Option<String>,
    pub ahead: Option<u64>,
    pub behind: Option<u64>,
    pub index: Vec<SourceControlEntry>,
    pub worktree: Vec<SourceControlEntry>,
    pub conflicts: Vec<SourceControlEntry>,
    /// Whether `git commit` would accept the index as it stands: it differs
    /// from HEAD, or a merge is being concluded (even one whose resolved tree
    /// equals HEAD). Decided by Git, not by the projected `index` list.
    pub committable: bool,
    pub omitted: SourceControlOmitted,
    /// True when Git's status output exceeded the bounded budget; the lists are incomplete.
    pub truncated: bool,
    pub captured_at_iso: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceControlStatusResult {
    pub repo_id: String,
    pub status: SourceControlStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceControlDiffResult {
    pub repo_id: String,
    pub path: String,
    pub area: SourceControlArea,
    pub patch: String,
    pub truncated: bool,
    pub binary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceControlActionResult {
    pub repo_id: String,
    pub kind: SourceControlActionKind,
    pub status: SourceControlStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit_sha: Option<String>,
}
