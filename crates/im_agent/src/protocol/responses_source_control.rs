// Path: crates/im_agent/src/protocol/responses_source_control.rs
// Description: Agent-to-UI source-control payloads: working-tree status, per-file diff, action outcome

use serde::{Deserialize, Serialize};

use super::commands_source_control::{
    SourceControlActionKind, SourceControlArea, SourceControlWorktreeStamp,
};

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
    /// Size and mtime of the file on disk, for worktree and conflict entries
    /// whose path exists. The UI returns it with a discard so the agent can
    /// refuse a file that changed after it was reviewed. Index entries never
    /// carry one: their content lives in the index, not on disk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_stamp: Option<SourceControlWorktreeStamp>,
    /// True for a worktree or conflict entry whose path Git reports changed
    /// but which is not currently on disk (a tracked deletion, or a file
    /// removed between the porcelain read and the stamp pass). The UI sends
    /// `expectedMissing: true` back with a discard target built from such an
    /// entry instead of a stamp, so the agent can refuse when a newer file has
    /// since appeared rather than silently restore over it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub worktree_missing: bool,
}

/// Changed paths that are counted but not listed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceControlOmitted {
    /// Staged paths outside the configured root (a subdirectory of the Git top
    /// level); a commit of the whole index carries them too. Worktree-only and
    /// untracked paths outside the root are not counted.
    pub staged_outside_root: u64,
    /// Unmerged paths outside the configured root; Git refuses a whole-index
    /// commit while any exist, so the UI must alert even though none is listed.
    pub unmerged_outside_root: u64,
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
    /// Whether `git commit` would accept the index as it stands: no unmerged
    /// record anywhere in the repository, and either the index differs from
    /// HEAD or a merge is being concluded (even one whose resolved tree equals
    /// HEAD). Decided by Git's own output, not by the projected `index` list.
    pub committable: bool,
    /// The tree id `git write-tree` would produce for the whole-repository
    /// index, computed read-only. Empty exactly when no candidate tree exists
    /// (the index holds unmerged entries). The UI returns it with a commit as
    /// the precondition that the index is still the one it reviewed.
    pub index_tree_sha: String,
    /// True when this repository's physical mutation lock was held while the
    /// status was read, so the lists may be mid-transaction.
    pub mutation_in_progress: bool,
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

/// Which Git snapshot one image-diff side was read from. Names the Git term
/// the UI pairs with its plain label (`PREVIOUS · HEAD`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageDiffSource {
    Head,
    Index,
    Worktree,
    Ours,
    Theirs,
}

/// One rendered snapshot of a changed image. `truncated` means the blob
/// exceeded the per-side bound: `data_base64` is empty and `bytes` reports the
/// bound, so the UI can say "too large to preview" instead of showing a broken
/// image. A side that does not exist is `None`, never an error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageDiffSide {
    pub source: ImageDiffSource,
    pub data_base64: String,
    pub mime_type: String,
    pub bytes: u64,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceControlImageDiffResult {
    pub repo_id: String,
    pub path: String,
    pub area: SourceControlArea,
    pub before: Option<ImageDiffSide>,
    pub after: Option<ImageDiffSide>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceControlActionResult {
    pub repo_id: String,
    pub kind: SourceControlActionKind,
    pub status: SourceControlStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit_sha: Option<String>,
    /// Paths a commit hook changed beyond the reviewed index, accepted because
    /// every one of them was already part of the paths the user reviewed (an
    /// in-root reviewed path, or an outside-root path staged at precondition
    /// time when the UI had shown its outside-root confirmation). Present only
    /// for a commit whose hook changed something; empty otherwise.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hook_changed_paths: Vec<String>,
}
