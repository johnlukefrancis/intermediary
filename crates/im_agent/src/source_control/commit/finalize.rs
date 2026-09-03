// Path: crates/im_agent/src/source_control/commit/finalize.rs
// Description: Post-commit comparison of the landed tree against the reviewed one, split into hook-changed and hook-added paths

use std::collections::HashSet;
use std::path::Path;

use crate::error::{AgentError, MutationEffect};
use crate::protocol::SourceControlStatus;

use crate::source_control::runner::{self, GitCall, INDEX_TIMEOUT, STATUS_LIMIT};

/// Git's own empty tree: the base the reviewed index tree is compared against
/// on what was an unborn branch, where there is no reviewed HEAD to diff from.
const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

/// A commit that landed, plus what a hook did to it beyond the tree the user
/// reviewed. Both lists are repository-root relative, the space `diff-tree`
/// reports in, and `None` when the hook did nothing of that kind.
pub(in crate::source_control) struct CommitOutcome {
    pub commit_sha: String,
    pub hook_changed_paths: Option<Vec<String>>,
    pub hook_added_paths: Option<Vec<String>>,
}

/// Runs once `git commit` has actually landed a commit (whether reported
/// directly or recovered after a timeout): compares the tree it produced
/// against the tree the user reviewed, and reports the difference.
///
/// The commit is never undone. It is history the moment Git wrote it —
/// hooks ran, other tools may already have seen it, and a ref rewind is a
/// second unreviewed mutation, not a cancellation of the first. What the user
/// is owed is the truth about what landed, so a divergent tree is split
/// against the reviewed paths: content the hook rewrote in paths the user
/// already had in front of them, and paths the hook brought in that nobody
/// reviewed.
pub(super) async fn finalize_commit(
    repo_root: &Path,
    reviewed: &SourceControlStatus,
) -> Result<CommitOutcome, AgentError> {
    let (new_head, new_tree) = head_and_tree(repo_root).await.map_err(head_unreadable)?;
    if new_tree == reviewed.index_tree_sha {
        return Ok(CommitOutcome {
            commit_sha: new_head,
            hook_changed_paths: None,
            hook_added_paths: None,
        });
    }
    let reviewed_paths = reviewed_paths(repo_root, reviewed)
        .await
        .map_err(head_unreadable)?;
    let changed = diff_tree_paths(repo_root, &reviewed.index_tree_sha, &new_tree)
        .await
        .map_err(head_unreadable)?;
    let (hook_changed, hook_added): (Vec<String>, Vec<String>) = changed
        .into_iter()
        .partition(|path| reviewed_paths.contains(path));
    Ok(CommitOutcome {
        commit_sha: new_head,
        hook_changed_paths: reported(hook_changed),
        hook_added_paths: reported(hook_added),
    })
}

/// Every path the reviewed commit was going to touch, read from the two
/// immutable objects this finalizer trusts as the reviewed state — the
/// reviewed index tree against the reviewed HEAD — never from the live index:
/// by the time this runs the commit has landed, so the index equals HEAD and
/// any read of it reports nothing at all, which would make every reviewed path
/// look like a hook's addition.
async fn reviewed_paths(
    repo_root: &Path,
    reviewed: &SourceControlStatus,
) -> Result<HashSet<String>, AgentError> {
    let base = reviewed.head_sha.as_deref().unwrap_or(EMPTY_TREE);
    let paths = diff_tree_paths(repo_root, base, &reviewed.index_tree_sha).await?;
    Ok(paths.into_iter().collect())
}

/// An empty list is not a fact worth putting on the wire: nothing changed of
/// that kind, and `None` says so without the UI having to test a length.
fn reported(paths: Vec<String>) -> Option<Vec<String>> {
    (!paths.is_empty()).then_some(paths)
}

/// A read needed to describe what the commit landed failed after the commit
/// itself had already landed: the outcome is unknown, not a Git failure — the
/// UI reconciles rather than reporting a failed commit.
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

/// `diff-tree` names paths from the repository top level whatever directory it
/// runs in, so both sides of the comparison — and the lists reported from it —
/// live in one path space regardless of which root sent the commit.
async fn diff_tree_paths(
    repo_root: &Path,
    from: &str,
    to: &str,
) -> Result<Vec<String>, AgentError> {
    let call = GitCall::new(["diff-tree", "-r", "--name-only", "-z", from, to])
        .stdout_limit(STATUS_LIMIT)
        .timeout(INDEX_TIMEOUT);
    let output = runner::run_read(repo_root, call, None).await?;
    Ok(split_nul_utf8(&output.stdout))
}

fn split_nul_utf8(bytes: &[u8]) -> Vec<String> {
    bytes
        .split(|byte| *byte == 0)
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
        .collect()
}
