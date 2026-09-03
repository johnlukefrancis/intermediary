// Path: crates/im_agent/src/source_control/actions_stage.rs
// Description: Stage and unstage one section or an explicit path list, never a pathspec wildcard

use std::path::Path;

use crate::error::{AgentError, MutationEffect};
use crate::protocol::{
    SourceControlChange, SourceControlEntry, SourceControlScope, SourceControlStatus,
};

use super::locks::SourceControlLocks;
use super::paths::{normalize_paths, nul_joined, PATHSPEC_FROM_STDIN};
use super::runner::{self, GitCall, INDEX_TIMEOUT};
use super::status;

/// `All` is the displayed section, not a directory: the agent re-reads status
/// under the mutation lock and passes exactly the paths that section holds, so
/// a bulk stage can never reach an unmerged path (those live in their own
/// section and are resolved one row at a time) and never reaches a path the
/// section did not list. Pathspec `.` is never issued.
pub(super) async fn stage(
    repo_root: &Path,
    scope: SourceControlScope,
    locks: &SourceControlLocks,
) -> Result<(), AgentError> {
    let paths = match scope {
        SourceControlScope::All => worktree_section(repo_root, locks).await?,
        SourceControlScope::Paths { paths } => explicit_paths(&paths)?,
    };
    apply(repo_root, GitCall::new(["add", "-A"]), paths).await
}

/// `reset` without a commit resolves to the empty tree on an unborn branch,
/// so unstaging works before the first commit.
pub(super) async fn unstage(
    repo_root: &Path,
    scope: SourceControlScope,
    locks: &SourceControlLocks,
) -> Result<(), AgentError> {
    let paths = match scope {
        SourceControlScope::All => index_section(repo_root, locks).await?,
        SourceControlScope::Paths { paths } => explicit_paths(&paths)?,
    };
    apply(repo_root, GitCall::new(["reset", "-q"]), paths).await
}

async fn apply(repo_root: &Path, call: GitCall, paths: Vec<String>) -> Result<(), AgentError> {
    if paths.is_empty() {
        // Only reachable from an empty section: zero pathspecs would mean the
        // whole repository to Git, so nothing is run.
        return Ok(());
    }
    let call = call
        .args(PATHSPEC_FROM_STDIN)
        .stdin(nul_joined(&paths))
        .timeout(INDEX_TIMEOUT);
    runner::run_mutation(repo_root, call)
        .await
        .map(drop)
        .map_err(|error| error.with_default_effect(MutationEffect::NotApplied))
}

/// The CHANGES section: every worktree entry, untracked files included,
/// conflicts excluded by construction.
async fn worktree_section(
    repo_root: &Path,
    locks: &SourceControlLocks,
) -> Result<Vec<String>, AgentError> {
    let status = section_status(repo_root, locks).await?;
    Ok(distinct(status.worktree.iter().map(|entry| entry.path.clone())))
}

/// The STAGED CHANGES section. A staged rename is one record with two index
/// endpoints, and unstaging it has to restore both, so a renamed entry
/// contributes its original path too. A copy contributes only its destination:
/// its source was never staged by the copy.
async fn index_section(
    repo_root: &Path,
    locks: &SourceControlLocks,
) -> Result<Vec<String>, AgentError> {
    let status = section_status(repo_root, locks).await?;
    Ok(distinct(status.index.iter().flat_map(rename_endpoints)))
}

fn rename_endpoints(entry: &SourceControlEntry) -> Vec<String> {
    let mut paths = vec![entry.path.clone()];
    if entry.change == SourceControlChange::Renamed {
        paths.extend(entry.original_path.clone());
    }
    paths
}

async fn section_status(
    repo_root: &Path,
    locks: &SourceControlLocks,
) -> Result<SourceControlStatus, AgentError> {
    status::capture_status(repo_root, None, locks)
        .await
        .map(|capture| capture.status)
        .map_err(|error| error.with_effect(MutationEffect::NotApplied))
}

fn distinct(paths: impl Iterator<Item = String>) -> Vec<String> {
    let mut collected: Vec<String> = paths.collect();
    collected.sort();
    collected.dedup();
    collected
}

/// An explicit list the UI sent. An empty one is a mistake, not a no-op: Git
/// would read zero pathspecs as the whole repository.
pub(super) fn explicit_paths(paths: &[String]) -> Result<Vec<String>, AgentError> {
    if paths.is_empty() {
        return Err(
            AgentError::new("INVALID_PATH", "No paths given")
                .with_effect(MutationEffect::NotApplied),
        );
    }
    normalize_paths(paths).map_err(|error| error.with_effect(MutationEffect::NotApplied))
}
