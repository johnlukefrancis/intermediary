// Path: crates/im_bundle/src/git_capture/mod.rs
// Description: Versioned selection-bounded Git evidence capture for bundle archives

mod command;
mod diff;
mod diff_issue;
mod discovery;
mod finalize;
mod ignored;
mod index;
mod index_tree;
mod initial_state;
mod path;
mod pathspec_batches;
mod porcelain;
mod render;
mod render_omitted;
mod session;
mod status;
mod verification;

use std::path::PathBuf;
use std::time::Duration;

use chrono::{SecondsFormat, Utc};
use serde::Serialize;

use crate::selection::BundleSelector;

use status::ParsedStatus;
pub(crate) use verification::WrittenEntryDigests;

pub const GIT_STATUS_NAME: &str = "BUNDLE_GIT_STATUS.txt";
pub const GIT_DIFF_NAME: &str = "BUNDLE_GIT_DIFF.patch";
pub const GIT_INDEX_DIFF_NAME: &str = "BUNDLE_GIT_INDEX_DIFF.patch";
pub const GIT_WORKTREE_DIFF_NAME: &str = "BUNDLE_GIT_WORKTREE_DIFF.patch";
pub const GIT_OMITTED_PATHS_NAME: &str = "BUNDLE_GIT_OMITTED_PATHS.txt";
pub const HANDOFF_NAME: &str = "BUNDLE_HANDOFF.md";
pub const GIT_CAPTURE_CONTRACT_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GitCaptureState {
    Complete,
    Partial,
    Unavailable,
    Unstable,
}

/// How deleted files appear in `BUNDLE_GIT_DIFF.patch`. `Full` keeps the
/// removed preimage; `HeaderOnly` is chosen only when a patch with deletion
/// bodies overran the reviewable budget, so the delta stays complete and
/// readable instead of truncated or dominated by removed content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PatchDeletions {
    Full,
    HeaderOnly,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCaptureCounts {
    pub selected_changed: u64,
    pub selected_tracked_changed: u64,
    pub selected_untracked: u64,
    pub selected_deleted: u64,
    pub selected_renamed: u64,
    pub selected_conflicted: u64,
    pub omitted_changed_paths: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitArtifactNames {
    pub status: String,
    pub diff: String,
    pub index_diff: String,
    pub worktree_diff: String,
    pub omitted_paths: String,
    pub handoff: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCaptureIssue {
    pub kind: String,
    pub artifact: Option<String>,
    pub detail: String,
}

impl GitCaptureIssue {
    pub(crate) fn new(kind: &str, artifact: Option<&str>, detail: &str) -> Self {
        Self {
            kind: kind.to_string(),
            artifact: artifact.map(str::to_string),
            detail: detail.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleGitCapture {
    pub contract_version: u32,
    pub comparison_base: String,
    pub captured_at: String,
    pub status: GitCaptureState,
    pub head_sha: Option<String>,
    pub short_sha: Option<String>,
    pub branch: Option<String>,
    /// Tree id the whole-repository index would commit as; the read-only
    /// equivalent of `git write-tree`. Absent when the index is unmerged or
    /// could not be listed.
    pub candidate_index_tree_sha: Option<String>,
    pub repo_dirty: Option<bool>,
    pub selection_dirty: Option<bool>,
    pub patch_deletions: PatchDeletions,
    pub counts: GitCaptureCounts,
    pub artifacts: GitArtifactNames,
    pub incomplete_artifacts: Vec<String>,
    pub issues: Vec<GitCaptureIssue>,
}

pub(crate) struct CapturedGitEvidence {
    pub(crate) manifest: BundleGitCapture,
    pub(crate) status: Vec<u8>,
    pub(crate) diff: Vec<u8>,
    pub(crate) index_diff: Vec<u8>,
    pub(crate) worktree_diff: Vec<u8>,
    pub(crate) omitted_paths: Vec<u8>,
    pub(crate) handoff: Vec<u8>,
}

pub(crate) struct GitCaptureSession {
    config: GitCaptureConfig,
    selector: BundleSelector,
    repo_prefix: Vec<u8>,
    pre_status_digest: Option<[u8; 32]>,
    initial_patch: Option<Vec<u8>>,
    initial_index_tree_sha: Option<String>,
    initial_digests: WrittenEntryDigests,
    initial_digests_complete: bool,
    selected_file_input: Option<Vec<u8>>,
    selected_file_paths: std::collections::HashSet<path::GitPath>,
    initial_ignored_paths: Option<Vec<path::GitPath>>,
    parsed_status: Option<ParsedStatus>,
    manifest: BundleGitCapture,
}

pub(crate) struct GitCaptureConfig {
    executable: PathBuf,
    repo_root: PathBuf,
    command_timeout: Duration,
    /// Hard output bound for `BUNDLE_GIT_DIFF.patch`; beyond it the patch is truncated.
    patch_limit: usize,
    /// Size past which a patch that includes deleted-file bodies is retried header-only.
    full_deletions_budget: usize,
}

fn empty_manifest() -> BundleGitCapture {
    BundleGitCapture {
        contract_version: GIT_CAPTURE_CONTRACT_VERSION,
        comparison_base: "HEAD".to_string(),
        captured_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        status: GitCaptureState::Unavailable,
        head_sha: None,
        short_sha: None,
        branch: None,
        candidate_index_tree_sha: None,
        repo_dirty: None,
        selection_dirty: None,
        patch_deletions: PatchDeletions::Full,
        counts: GitCaptureCounts::default(),
        artifacts: GitArtifactNames {
            status: GIT_STATUS_NAME.to_string(),
            diff: GIT_DIFF_NAME.to_string(),
            index_diff: GIT_INDEX_DIFF_NAME.to_string(),
            worktree_diff: GIT_WORKTREE_DIFF_NAME.to_string(),
            omitted_paths: GIT_OMITTED_PATHS_NAME.to_string(),
            handoff: HANDOFF_NAME.to_string(),
        },
        incomplete_artifacts: Vec::new(),
        issues: Vec::new(),
    }
}

#[cfg(test)]
mod tests;
