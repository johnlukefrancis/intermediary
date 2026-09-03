// Path: crates/im_agent/src/source_control/actions_commit_retract.rs
// Description: Post-commit tree comparison against the reviewed state, and the CAS retraction of a hook overreach

use std::collections::HashSet;
use std::path::Path;

use crate::error::{AgentError, MutationEffect};
use crate::protocol::SourceControlStatus;

use super::runner::{self, GitCall, INDEX_TIMEOUT, STATUS_LIMIT};

/// Git's own empty tree: the base the reviewed index tree is compared against
/// on what was an unborn branch, where there is no reviewed HEAD to diff from.
const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

/// A commit that landed, plus whatever a hook changed beyond the reviewed
/// index (empty when the committed tree matched exactly).
pub(super) struct CommitOutcome {
    pub commit_sha: String,
    pub hook_changed_paths: Vec<String>,
}

/// Runs once `git commit` has actually landed a commit (whether reported
/// directly or recovered after a timeout): compares the tree it produced
/// against the tree the user reviewed. Equal trees mean the hooks changed
/// nothing observable; a different tree is inspected path by path against
/// everything the user reviewed (the in-root staged list, plus — when the
/// precondition status showed staged content outside the configured root —
/// every path staged anywhere in the repository at that same precondition
/// read). Every changed path inside that reviewed set is accepted as the
/// hook behaviour the design allows (lint-staged reformatting what it just
/// committed); anything outside it is retracted.
pub(super) async fn finalize_commit(
    repo_root: &Path,
    prefix: &[u8],
    expected_tree: &str,
    expected_head_sha: Option<&str>,
    reviewed_status: &SourceControlStatus,
) -> Result<CommitOutcome, AgentError> {
    let (new_head, new_tree) = head_and_tree(repo_root).await.map_err(head_unreadable)?;
    if new_tree == expected_tree {
        return Ok(CommitOutcome {
            commit_sha: new_head,
            hook_changed_paths: Vec::new(),
        });
    }
    let diff_paths = diff_tree_paths(repo_root, expected_tree, &new_tree)
        .await
        .map_err(head_unreadable)?;
    let allowed = allowed_paths(
        repo_root,
        prefix,
        expected_tree,
        expected_head_sha,
        reviewed_status,
    )
    .await
    .map_err(head_unreadable)?;
    if diff_paths.iter().all(|path| allowed.contains(path)) {
        return Ok(CommitOutcome {
            commit_sha: new_head,
            hook_changed_paths: diff_paths,
        });
    }
    retract(repo_root, expected_head_sha, &new_head, &diff_paths).await
}

/// A read needed to decide whether the commit was clean failed after the
/// commit itself had already landed: the outcome is unknown, not a Git
/// failure — the UI reconciles rather than reporting a failed commit.
fn head_unreadable(inner: AgentError) -> AgentError {
    AgentError::new(
        "ACTION_APPLIED_STATUS_UNAVAILABLE",
        format!(
            "commit completed but its resulting state could not be read: {}",
            inner.message()
        ),
    )
    .with_effect(MutationEffect::Unknown)
}

async fn head_and_tree(repo_root: &Path) -> Result<(String, String), AgentError> {
    let call = GitCall::new(["rev-parse", "HEAD", "HEAD^{tree}"]);
    let output = runner::run_read(repo_root, call, None).await?;
    let text = String::from_utf8_lossy(&output.stdout);
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let (Some(head), Some(tree)) = (lines.next(), lines.next()) else {
        return Err(AgentError::new(
            "GIT_COMMAND_FAILED",
            "Git did not report both HEAD and its tree",
        ));
    };
    Ok((head.to_string(), tree.to_string()))
}

async fn diff_tree_paths(
    repo_root: &Path,
    expected_tree: &str,
    actual_tree: &str,
) -> Result<Vec<String>, AgentError> {
    let call = GitCall::new(["diff-tree", "-r", "--name-only", "-z", expected_tree, actual_tree])
        .stdout_limit(STATUS_LIMIT)
        .timeout(INDEX_TIMEOUT);
    let output = runner::run_read(repo_root, call, None).await?;
    Ok(split_nul_utf8(&output.stdout))
}

/// Every path the user reviewed at the precondition read, in git-top-level
/// path space (the same space `diff-tree` reports in): the in-root staged
/// list, recombined with the configured root's prefix, and — only when that
/// same read showed staged content outside the root, i.e. only when the UI's
/// outside-root confirmation was the reason the click was allowed to send at
/// all — every path staged anywhere in the repository at that moment.
///
/// That whole-repository set is read from the two immutable objects this
/// finalizer already trusts as the reviewed state — the reviewed index tree
/// against the reviewed HEAD — never from the live index: by the time this
/// runs the commit has landed, so the index equals HEAD and any read of it
/// reports nothing at all, which would make every acknowledged outside-root
/// path look unreviewed and retract a commit the user confirmed.
async fn allowed_paths(
    repo_root: &Path,
    prefix: &[u8],
    expected_tree: &str,
    expected_head_sha: Option<&str>,
    reviewed_status: &SourceControlStatus,
) -> Result<HashSet<String>, AgentError> {
    let mut allowed = HashSet::new();
    for entry in &reviewed_status.index {
        allowed.insert(with_prefix(prefix, &entry.path));
        if let Some(original) = &entry.original_path {
            allowed.insert(with_prefix(prefix, original));
        }
    }
    if reviewed_status.omitted.staged_outside_root > 0 {
        let reviewed_head = expected_head_sha.unwrap_or(EMPTY_TREE);
        allowed.extend(diff_tree_paths(repo_root, reviewed_head, expected_tree).await?);
    }
    Ok(allowed)
}

/// `prefix` already carries its trailing slash (empty at the top level).
fn with_prefix(prefix: &[u8], path: &str) -> String {
    if prefix.is_empty() {
        return path.to_string();
    }
    format!("{}{path}", String::from_utf8_lossy(prefix))
}

fn split_nul_utf8(bytes: &[u8]) -> Vec<String> {
    bytes
        .split(|byte| *byte == 0)
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
        .collect()
}

/// The hook staged something the user never reviewed: the commit is undone by
/// a compare-and-swap ref update rather than left standing, so the repository
/// ends up exactly as if the commit had been refused up front. `expected_head`
/// is `None` on what was an unborn branch, in which case "undoing" the ref is
/// deleting it (still a CAS: only if it still names the commit just made).
async fn retract(
    repo_root: &Path,
    expected_head: Option<&str>,
    new_head: &str,
    diff_paths: &[String],
) -> Result<CommitOutcome, AgentError> {
    let ref_name = resolve_head_ref(repo_root)
        .await
        .unwrap_or_else(|_| "HEAD".to_string());
    let call = match expected_head {
        Some(previous) => GitCall::new(["update-ref", &ref_name, previous, new_head]),
        None => GitCall::new(["update-ref", "-d", &ref_name, new_head]),
    };
    match runner::run_mutation(repo_root, call.timeout(INDEX_TIMEOUT)).await {
        Ok(_) => Err(AgentError::new(
            "SOURCE_CONTROL_STATE_CHANGED",
            format!(
                "a commit hook staged unreviewed paths: {}",
                diff_paths.join(", ")
            ),
        )
        .with_effect(MutationEffect::NotApplied)),
        Err(error) => Err(AgentError::new(
            "GIT_COMMAND_FAILED",
            format!(
                "a commit hook staged unreviewed paths and commit {new_head} could not be retracted: {}",
                error.message()
            ),
        )
        .with_effect(MutationEffect::Unknown)),
    }
}

/// `symbolic-ref` names the branch HEAD points at (exit 1 on a detached
/// HEAD, accepted); an attached branch is updated by name so the retraction
/// moves the branch pointer, not a detached `HEAD` file that nothing tracks.
async fn resolve_head_ref(repo_root: &Path) -> Result<String, AgentError> {
    let call = GitCall::new(["symbolic-ref", "-q", "HEAD"]).accept_exit_codes(&[1]);
    let output = runner::run_read(repo_root, call, None).await?;
    if output.exit_code == 1 {
        return Ok("HEAD".to_string());
    }
    let name = String::from_utf8_lossy(&im_bundle::git::trim_line_ending(output.stdout))
        .trim()
        .to_string();
    Ok(if name.is_empty() { "HEAD".to_string() } else { name })
}
