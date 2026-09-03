// Path: crates/im_agent/src/source_control/actions_commit.rs
// Description: Commit under the reviewed index+HEAD precondition, with timeout recovery and hook finalization

use std::path::Path;

use im_bundle::git::trim_line_ending;

use crate::error::{AgentError, MutationEffect};

use super::actions_commit_retract::{finalize_commit, CommitOutcome};
use super::locks::SourceControlLocks;
use super::runner::{self, GitCall, COMMIT_TIMEOUT};
use super::status;

/// Commits the whole index (a partial `commit -- <paths>` is never issued).
///
/// The state the user reviewed is the state that gets committed: the agent
/// re-reads status under the mutation lock and refuses when the index or HEAD
/// identity has moved, so a file staged — or a commit landed — between the
/// review and the click cannot ride along unseen. Once `git commit` itself
/// has run, `finalize_commit` re-checks the actual result against that same
/// reviewed state: a hook that only touched paths the user already reviewed
/// is accepted, anything else is retracted. Unresolved conflicts are refused
/// with their own code, because Git would refuse them too and the user has to
/// resolve rows, not retry.
pub(super) async fn commit(
    repo_root: &Path,
    message: &str,
    expected_index_tree_sha: &str,
    expected_head_sha: Option<&str>,
    locks: &SourceControlLocks,
) -> Result<CommitOutcome, AgentError> {
    if message.trim().is_empty() {
        return Err(refusal(
            "INVALID_COMMIT_MESSAGE",
            "Commit message must not be blank".to_string(),
        ));
    }
    let location = runner::capture_location(repo_root, None).await?;
    let capture = status::capture_status(repo_root, None, locks)
        .await
        .map_err(|error| error.with_effect(MutationEffect::NotApplied))?;
    // Conflicts come first: an unmerged index has no candidate tree at all, so
    // its identity reads empty and every other comparison below would report
    // "the index moved" for what is really "resolve these rows".
    if capture.unmerged {
        return Err(refusal(
            "GIT_UNMERGED_PATHS",
            "Unresolved conflicts remain; resolve them before committing".to_string(),
        ));
    }
    // An empty identity is not a sha nobody matched, it is "this index had no
    // stable identity when it was read" — a torn status read. Two empties would
    // otherwise compare equal and authorize a commit of a tree nobody reviewed,
    // so it is refused rather than compared.
    if expected_index_tree_sha.is_empty() {
        return Err(refusal(
            "SOURCE_CONTROL_STATE_CHANGED",
            "the reviewed index had no stable identity; refresh and review again".to_string(),
        ));
    }
    if capture.status.index_tree_sha != expected_index_tree_sha {
        return Err(refusal(
            "SOURCE_CONTROL_STATE_CHANGED",
            "index changed since it was reviewed".to_string(),
        ));
    }
    if capture.status.head_sha.as_deref() != expected_head_sha {
        return Err(refusal(
            "SOURCE_CONTROL_STATE_CHANGED",
            "HEAD changed since it was reviewed".to_string(),
        ));
    }
    if !capture.status.committable {
        return Err(refusal(
            "GIT_NOTHING_TO_COMMIT",
            "Nothing is staged to commit".to_string(),
        ));
    }
    let call = GitCall::new(["commit", "-q", "--cleanup=whitespace", "-F", "-"])
        .stdin(message.as_bytes().to_vec())
        .timeout(COMMIT_TIMEOUT);
    match runner::run_mutation(repo_root, call).await {
        // `git commit` reports a non-zero exit only when no commit was made;
        // a hook that fails after the commit object exists does not change it.
        Ok(_) => {
            finalize_commit(
                repo_root,
                &location.prefix,
                expected_index_tree_sha,
                expected_head_sha,
                &capture.status,
            )
            .await
        }
        Err(error) => {
            recover_if_head_moved(
                repo_root,
                &location.prefix,
                expected_index_tree_sha,
                expected_head_sha,
                &capture.status,
                error.with_default_effect(MutationEffect::NotApplied),
            )
            .await
        }
    }
}

fn refusal(code: &str, message: String) -> AgentError {
    AgentError::new(code, message).with_effect(MutationEffect::NotApplied)
}

/// A commit whose hook outran the command bound has still landed. Git's own
/// error code says which layer spoke, never whether the repository changed, so
/// the only honest answer is to look at HEAD: moved means committed, and from
/// there the same finalization the success path takes decides whether the
/// hook stayed inside the reviewed state.
async fn recover_if_head_moved(
    repo_root: &Path,
    prefix: &[u8],
    expected_index_tree_sha: &str,
    expected_head_sha: Option<&str>,
    reviewed_status: &crate::protocol::SourceControlStatus,
    error: AgentError,
) -> Result<CommitOutcome, AgentError> {
    if error.effect() == Some(MutationEffect::NotApplied.as_str()) {
        return Err(error);
    }
    match head_sha(repo_root).await {
        Ok(after) if after.is_some() && after.as_deref() != expected_head_sha => {
            finalize_commit(
                repo_root,
                prefix,
                expected_index_tree_sha,
                expected_head_sha,
                reviewed_status,
            )
            .await
        }
        _ => Err(error),
    }
}

/// The current HEAD commit, or `None` on an unborn branch.
pub(super) async fn head_sha(repo_root: &Path) -> Result<Option<String>, AgentError> {
    let call = GitCall::new(["rev-parse", "-q", "--verify", "HEAD"]).accept_exit_codes(&[1]);
    let output = runner::run_read(repo_root, call, None).await?;
    let sha = String::from_utf8_lossy(&trim_line_ending(output.stdout))
        .trim()
        .to_string();
    Ok((!sha.is_empty()).then_some(sha))
}
