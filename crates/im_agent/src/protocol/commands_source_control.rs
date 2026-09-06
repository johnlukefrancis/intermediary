// Path: crates/im_agent/src/protocol/commands_source_control.rs
// Description: UI-to-agent source-control command payloads (status, diff, tagged actions)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceControlStatusCommand {
    pub repo_id: String,
}

/// Which side of the index a diff or entry refers to. `Index` compares HEAD to
/// the index (staged), `Worktree` compares the index to the working tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceControlArea {
    Index,
    Worktree,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceControlDiffCommand {
    pub repo_id: String,
    pub path: String,
    /// Rename source for renamed/copied entries so the diff can pair both paths.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_path: Option<String>,
    pub area: SourceControlArea,
}

/// Both snapshots of one changed image. `original_path` names the rename
/// source so the HEAD side of a renamed entry resolves to the old path.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceControlImageDiffCommand {
    pub repo_id: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_path: Option<String>,
    pub area: SourceControlArea,
}

/// Pathspec scope for stage/unstage. `All` names the whole displayed section:
/// the agent enumerates it from a fresh status and passes those paths
/// explicitly, so a bulk stage never crosses into unmerged paths and never
/// reaches beyond what the section listed. `Paths` with an empty list is
/// rejected by the agent before any process spawns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum SourceControlScope {
    All,
    Paths { paths: Vec<String> },
}

/// Size and modification time of one worktree file, as the agent read them.
/// The UI sends back the stamp it displayed so a discard can refuse a file that
/// changed after the user confirmed it. Millisecond resolution matches what a
/// browser reports; `mtime_nanos` is the agent's own finer-grained read (the
/// nanosecond-of-second component `fs::metadata` returns) and is compared only
/// between two agent-reported stamps — a discard refuses on either field
/// mismatching, not just the millisecond one, so a same-length rewrite that
/// happens to land in the same millisecond is still caught.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceControlWorktreeStamp {
    pub bytes: u64,
    pub mtime_ms: i64,
    pub mtime_nanos: u32,
}

/// One file a discard is allowed to touch. `expected_stamp` is the file's stamp
/// when it existed at review time; `expected_missing` is `true` when the
/// reviewed status showed the file absent (`worktreeMissing`) — the agent then
/// refuses if a newer file has since appeared, rather than silently restoring
/// over it. The two are mutually exclusive.
///
/// A target carries neither only where the review had no assertion to make
/// about a file's bytes: the second endpoint of a rename row (which has no
/// status entry of its own), and an entry whose path is not a regular file (a
/// directory or symlink, which is never stamped). Such a target is only
/// restored, never removed, and is never claimed into quarantine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceControlDiscardTarget {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_stamp: Option<SourceControlWorktreeStamp>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub expected_missing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SourceControlActionPayload {
    Stage {
        scope: SourceControlScope,
    },
    Unstage {
        scope: SourceControlScope,
    },
    /// Only the listed targets are touched. A copy row sends its destination
    /// alone; the agent never expands a record's provenance into a target.
    Discard {
        targets: Vec<SourceControlDiscardTarget>,
    },
    /// `expected_snapshot_id` is the `snapshotId` of the status the user
    /// reviewed: one identity covering branch, HEAD, index and merge state.
    /// The agent refuses the commit when it no longer matches the repository,
    /// and refuses an empty one outright rather than comparing it.
    Commit {
        message: String,
        expected_snapshot_id: String,
    },
    Push,
    Pull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceControlActionKind {
    Stage,
    Unstage,
    Discard,
    Commit,
    Push,
    Pull,
}

impl SourceControlActionPayload {
    pub fn kind(&self) -> SourceControlActionKind {
        match self {
            Self::Stage { .. } => SourceControlActionKind::Stage,
            Self::Unstage { .. } => SourceControlActionKind::Unstage,
            Self::Discard { .. } => SourceControlActionKind::Discard,
            Self::Commit { .. } => SourceControlActionKind::Commit,
            Self::Push => SourceControlActionKind::Push,
            Self::Pull => SourceControlActionKind::Pull,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceControlActionCommand {
    pub repo_id: String,
    pub action: SourceControlActionPayload,
}
