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
//! physical Git directory through `SourceControlLocks`.
//!
//! Outcome contract: every failed mutation carries `details.effect`. The error
//! code says which layer spoke; only `effect` says whether the repository
//! changed, and it is `notApplied` only where a site proved it.

mod actions;
mod commit;
mod diff;
mod discard;
mod locks;
mod paths;
mod runner;
mod status;
#[cfg(test)]
mod tests_support;

use std::path::Path;

use im_bundle::cancel::BundleCancelToken;

use crate::error::{AgentError, MutationEffect};
use crate::protocol::{
    ImageDiffSide, SourceControlActionPayload, SourceControlArea, SourceControlStatus,
};

pub use locks::{MutationGuard, SourceControlLocks};
/// The in-root containment guard, shared with the import path: both write to
/// the worktree without Git, so both owe the same proof about where a
/// relative path actually lands.
pub(crate) use paths::{ensure_no_git_component, ensure_within_root, normalize_path};
/// Removing worktree entries recoverably, shared with the worktree actions:
/// a delete is a quarantine claim, and the quarantine — its markers, its live
/// operation registry, its startup sweep — is owned here.
pub(crate) use discard::quarantine_entries;

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
/// new HEAD for commits and, for a commit a hook touched, what it did — the
/// reviewed paths it rewrote and the unreviewed paths it added. Both are
/// `None` for every other action kind, and for a commit no hook touched.
#[derive(Debug, Clone)]
pub struct SourceControlActionOutcome {
    pub status: SourceControlStatus,
    pub commit_sha: Option<String>,
    pub hook_changed_paths: Option<Vec<String>>,
    pub hook_added_paths: Option<Vec<String>>,
}

/// Whole-repository status projected onto the configured root. The locks
/// registry is read, never taken: a status read must not queue behind a
/// mutation, it reports that one is running.
pub async fn source_control_status(
    repo_root: &Path,
    cancel_token: Option<BundleCancelToken>,
    locks: &SourceControlLocks,
) -> Result<SourceControlStatus, AgentError> {
    status::capture_status(repo_root, cancel_token, locks)
        .await
        .map(|capture| capture.status)
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
    diff::image::capture_image_diff(repo_root, path, original_path, area, cancel_token).await
}

/// Runs one mutation under the lock of the physical Git directory that owns
/// `repo_root`, and returns the status read immediately afterwards. The guard
/// is released when this future ends, however it ends.
///
/// Every error leaving here carries an effect. Sites closer to the Git process
/// prove `notApplied` where they can; anything unclassified is `unknown`, so a
/// failure this owner did not anticipate makes the UI reconcile instead of
/// reporting a certainty nobody established.
pub async fn run_source_control_action(
    locks: &SourceControlLocks,
    repo_root: &Path,
    action: SourceControlActionPayload,
) -> Result<SourceControlActionOutcome, AgentError> {
    let _guard = locks
        .acquire(repo_root)
        .await
        .map_err(|error| error.with_default_effect(MutationEffect::NotApplied))?;
    actions::run_action(repo_root, action, locks)
        .await
        .map_err(|error| error.with_default_effect(MutationEffect::Unknown))
}
