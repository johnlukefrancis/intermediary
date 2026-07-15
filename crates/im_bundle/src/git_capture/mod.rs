// Path: crates/im_bundle/src/git_capture/mod.rs
// Description: Versioned selection-bounded Git evidence capture for bundle archives

mod command;
mod diff;
mod diff_issue;
mod discovery;
mod finalize;
mod ignored;
mod path;
mod pathspec_batches;
mod porcelain;
mod render;
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
pub const HANDOFF_NAME: &str = "BUNDLE_HANDOFF.md";
pub const GIT_CAPTURE_CONTRACT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GitCaptureState {
    Complete,
    Partial,
    Unavailable,
    Unstable,
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
    pub repo_dirty: Option<bool>,
    pub selection_dirty: Option<bool>,
    pub counts: GitCaptureCounts,
    pub artifacts: GitArtifactNames,
    pub incomplete_artifacts: Vec<String>,
    pub issues: Vec<GitCaptureIssue>,
}

pub(crate) struct CapturedGitEvidence {
    pub(crate) manifest: BundleGitCapture,
    pub(crate) status: Vec<u8>,
    pub(crate) diff: Vec<u8>,
    pub(crate) handoff: Vec<u8>,
}

pub(crate) struct GitCaptureSession {
    config: GitCaptureConfig,
    selector: BundleSelector,
    repo_prefix: Vec<u8>,
    pre_status_digest: Option<[u8; 32]>,
    initial_patch: Option<Vec<u8>>,
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
        repo_dirty: None,
        selection_dirty: None,
        counts: GitCaptureCounts::default(),
        artifacts: GitArtifactNames {
            status: GIT_STATUS_NAME.to_string(),
            diff: GIT_DIFF_NAME.to_string(),
            handoff: HANDOFF_NAME.to_string(),
        },
        incomplete_artifacts: Vec::new(),
        issues: Vec::new(),
    }
}

#[cfg(test)]
mod tests;
