// Path: crates/im_agent/src/source_control/mod.rs
// Description: Git working-tree status, per-file diff, and index/commit/remote actions for one repo root

//! Source control runs Git in the agent that owns the repo root: the Windows
//! host agent for host roots, the in-WSL agent for WSL roots. All Git work is
//! blocking and runs on `spawn_blocking` (ADR-009).
//!
//! Cancellation contract: reads (`status`, `diff`) take a cancel token and may
//! be killed. Mutations (`run_action`) deliberately take no token: a killed
//! `git commit` or `git add` bypasses Git's lockfile cleanup and leaves
//! `.git/index.lock` behind, wedging the repo for every tool. Mutations are
//! bounded by their timeout, use a graceful kill policy, and are serialized per
//! repo through `SourceControlLocks`.

mod actions;
mod actions_discard;
mod diff;
mod git_version;
mod image_diff;
mod image_diff_sides;
mod locks;
mod paths;
mod runner;
mod status;
mod status_project;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_actions;
#[cfg(test)]
mod tests_commit;
#[cfg(test)]
mod tests_diff;
#[cfg(test)]
mod tests_image_diff;
#[cfg(test)]
mod tests_support;

use std::path::Path;

use im_bundle::cancel::BundleCancelToken;

use crate::error::AgentError;
use crate::protocol::{
    ImageDiffSide, SourceControlActionPayload, SourceControlArea, SourceControlStatus,
};

pub use locks::SourceControlLocks;

/// A captured per-file patch. `truncated` means the bounded output budget was
/// exhausted; `binary` means Git reported a binary difference and `patch`
/// holds only Git's summary line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceControlDiff {
    pub patch: String,
    pub truncated: bool,
    pub binary: bool,
}

/// Both snapshots of one changed image. Either side is `None` when that
/// snapshot does not exist (added, deleted, unborn HEAD, missing merge stage);
/// a side past the per-side bound arrives `truncated` with no bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceControlImageDiff {
    pub before: Option<ImageDiffSide>,
    pub after: Option<ImageDiffSide>,
}

/// Outcome of a mutation: the fresh status read after the action, plus the
/// new HEAD for commits.
#[derive(Debug, Clone)]
pub struct SourceControlActionOutcome {
    pub status: SourceControlStatus,
    pub commit_sha: Option<String>,
}

/// Whole-repository status projected onto the configured root.
pub async fn source_control_status(
    repo_root: &Path,
    cancel_token: Option<BundleCancelToken>,
) -> Result<SourceControlStatus, AgentError> {
    status::capture_status(repo_root, cancel_token).await
}

/// Bounded unified diff for one repo-root-relative path in one area.
pub async fn source_control_diff(
    repo_root: &Path,
    path: &str,
    original_path: Option<&str>,
    area: SourceControlArea,
    cancel_token: Option<BundleCancelToken>,
) -> Result<SourceControlDiff, AgentError> {
    diff::capture_diff(repo_root, path, original_path, area, cancel_token).await
}

/// Bounded before/after image snapshots for one repo-root-relative path. The
/// index decides a conflict; the requested area decides everything else.
pub async fn source_control_image_diff(
    repo_root: &Path,
    path: &str,
    original_path: Option<&str>,
    area: SourceControlArea,
    cancel_token: Option<BundleCancelToken>,
) -> Result<SourceControlImageDiff, AgentError> {
    image_diff::capture_image_diff(repo_root, path, original_path, area, cancel_token).await
}

/// Runs one mutation under the repo's mutation lock and returns the status
/// read immediately afterwards.
pub async fn run_source_control_action(
    locks: &SourceControlLocks,
    repo_id: &str,
    repo_root: &Path,
    action: SourceControlActionPayload,
) -> Result<SourceControlActionOutcome, AgentError> {
    let repo_lock = locks.lock_for(repo_id);
    let _guard = repo_lock.lock().await;
    actions::run_action(repo_root, action).await
}
