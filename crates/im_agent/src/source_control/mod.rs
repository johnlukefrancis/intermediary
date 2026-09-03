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
mod actions_commit;
mod actions_commit_retract;
mod actions_discard;
mod actions_discard_claim;
mod actions_discard_target;
mod actions_remote;
mod actions_stage;
mod diff;
mod discard_quarantine;
mod git_version;
mod locks;
mod paths;
mod runner;
mod runner_failure;
mod status;
mod status_index_tree;
mod status_project;
mod status_stamp;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_actions;
#[cfg(test)]
mod tests_commit;
#[cfg(test)]
mod tests_commit_hooks;
#[cfg(test)]
mod tests_diff;
#[cfg(test)]
mod tests_discard_quarantine;
#[cfg(test)]
mod tests_discard_stamps;
#[cfg(test)]
mod tests_locks;
#[cfg(test)]
mod tests_preconditions;
#[cfg(test)]
mod tests_support;

use std::path::Path;

use im_bundle::cancel::BundleCancelToken;

use crate::error::{AgentError, MutationEffect};
use crate::protocol::{SourceControlActionPayload, SourceControlArea, SourceControlStatus};

pub use locks::{MutationGuard, SourceControlLocks};

/// A captured per-file patch. `truncated` means the bounded output budget was
/// exhausted; `binary` means Git reported a binary difference and `patch`
/// holds only Git's summary line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceControlDiff {
    pub patch: String,
    pub truncated: bool,
    pub binary: bool,
}

/// Outcome of a mutation: the fresh status read after the action, plus the
/// new HEAD for commits and, for a commit whose hook changed anything, the
/// paths it changed (empty for every other action kind).
#[derive(Debug, Clone)]
pub struct SourceControlActionOutcome {
    pub status: SourceControlStatus,
    pub commit_sha: Option<String>,
    pub hook_changed_paths: Vec<String>,
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
