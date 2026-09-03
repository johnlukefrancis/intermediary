// Path: crates/im_agent/src/source_control/actions_discard.rs
// Description: Discard worktree changes: restore tracked paths through Git, remove untracked regular files in Rust

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use crate::error::AgentError;
use crate::protocol::{SourceControlChange, SourceControlStatus};

use super::paths::{normalize_paths, nul_joined, resolve_untracked_file, PATHSPEC_FROM_STDIN};
use super::runner::{self, GitCall, INDEX_TIMEOUT};
use super::status;

/// Each requested path is classified by a fresh status read: an untracked
/// file is removed in Rust; an intent-to-add file (`git add -N`) leaves the
/// index first, then is removed; every other worktree or conflict entry goes
/// to `restore --worktree`; a path the worktree lists do not mention is a
/// validated no-op. Every removal target is resolved (regular file, inside the
/// root) before any mutation, so one refused path aborts the discard untouched.
pub(super) async fn discard(repo_root: &Path, paths: &[String]) -> Result<(), AgentError> {
    if paths.is_empty() {
        return Ok(());
    }
    let paths = normalize_paths(paths)?;
    let status = status::capture_status(repo_root, None).await?;
    let plan = DiscardPlan::classify(&status, paths);
    let targets = resolve_targets(repo_root, plan.remove).await?;
    if !plan.restore.is_empty() {
        let call = GitCall::new(["restore", "--worktree"])
            .args(PATHSPEC_FROM_STDIN)
            .stdin(nul_joined(&plan.restore))
            .timeout(INDEX_TIMEOUT);
        runner::run_mutation(repo_root, call).await?;
    }
    if !plan.unstage.is_empty() {
        let call = GitCall::new(["reset", "-q"])
            .args(PATHSPEC_FROM_STDIN)
            .stdin(nul_joined(&plan.unstage))
            .timeout(INDEX_TIMEOUT);
        runner::run_mutation(repo_root, call).await?;
    }
    remove_targets(targets).await
}

#[derive(Default)]
struct DiscardPlan {
    restore: Vec<String>,
    unstage: Vec<String>,
    remove: Vec<String>,
}

impl DiscardPlan {
    fn classify(status: &SourceControlStatus, paths: Vec<String>) -> Self {
        let listed: HashMap<&str, SourceControlChange> = status
            .worktree
            .iter()
            .chain(&status.conflicts)
            .map(|entry| (entry.path.as_str(), entry.change))
            .collect();
        let mut plan = Self::default();
        for path in paths {
            match listed.get(path.as_str()) {
                Some(SourceControlChange::Untracked) => plan.remove.push(path),
                Some(SourceControlChange::Added) => {
                    plan.unstage.push(path.clone());
                    plan.remove.push(path);
                }
                Some(_) => plan.restore.push(path),
                None => {}
            }
        }
        plan
    }
}

async fn resolve_targets(
    repo_root: &Path,
    untracked: Vec<String>,
) -> Result<Vec<PathBuf>, AgentError> {
    if untracked.is_empty() {
        return Ok(Vec::new());
    }
    let repo_root = repo_root.to_path_buf();
    blocking(move || {
        untracked
            .iter()
            .filter_map(|path| resolve_untracked_file(&repo_root, path).transpose())
            .collect()
    })
    .await
}

async fn remove_targets(targets: Vec<PathBuf>) -> Result<(), AgentError> {
    if targets.is_empty() {
        return Ok(());
    }
    blocking(move || {
        for target in &targets {
            match std::fs::remove_file(target) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(AgentError::internal(format!(
                        "Failed to discard {}: {error}",
                        target.display()
                    )))
                }
            }
        }
        Ok(())
    })
    .await
}

async fn blocking<T, F>(work: F) -> Result<T, AgentError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, AgentError> + Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .unwrap_or_else(|error| Err(AgentError::internal(format!("Discard task failed: {error}"))))
}
