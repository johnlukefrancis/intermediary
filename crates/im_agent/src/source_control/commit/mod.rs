// Path: crates/im_agent/src/source_control/commit/mod.rs
// Description: Commit under the reviewed-snapshot precondition, with timeout recovery and hook reporting

mod finalize;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_hooks;
#[cfg(test)]
mod tests_preconditions;

use std::path::Path;

use im_bundle::git::trim_line_ending;

use crate::error::{AgentError, MutationEffect};
use crate::protocol::SourceControlStatus;

use self::finalize::{finalize_commit, CommitOutcome};
use crate::source_control::locks::SourceControlLocks;
use crate::source_control::runner::{self, GitCall, COMMIT_TIMEOUT};
use crate::source_control::status;

/// Commits the whole index (a partial `commit -- <paths>` is never issued).
///
/// The state the user reviewed is the state that gets committed: the agent
/// re-reads status under the mutation lock and refuses when the snapshot
/// identity has moved, so a file staged, a commit landed, a branch switched or
/// a merge started between the review and the click cannot ride along unseen.
/// One identity covers all of them — there is no state a commit depends on
/// that the precondition does not see.
///
/// Once `git commit` has run the commit is history; `finalize_commit` reports
/// what a hook did to it rather than undoing anything. Unresolved conflicts are
/// refused with their own code, because Git would refuse them too and the user
/// has to resolve rows, not retry.
pub(super) async fn commit(
    repo_root: &Path,
    message: &str,
    expected_snapshot_id: &str,
    locks: &SourceControlLocks,
) -> Result<CommitOutcome, AgentError> {
    if message.trim().is_empty() {
        return Err(refusal(
            "INVALID_COMMIT_MESSAGE",
            "Commit message must not be blank".to_string(),
        ));
    }
    let capture = status::capture_status(repo_root, None, locks)
        .await
        .map_err(|error| error.with_effect(MutationEffect::NotApplied))?;
    // Conflicts come first: an unmerged index has no candidate tree at all, so
    // its snapshot reads empty and the refusal below would report "refresh and
    // retry" for what is really "resolve these rows".
    if capture.unmerged {
        return Err(refusal(
            "GIT_UNMERGED_PATHS",
            "Unresolved conflicts remain; resolve them before committing".to_string(),
        ));
    }
    // An empty identity is not a snapshot nobody matched, it is "no snapshot was
    // taken" — a torn index read, or state this agent could not read. Two
    // empties would otherwise compare equal and authorize a commit of a
    // repository nobody reviewed, so it is refused rather than compared.
    if expected_snapshot_id.is_empty() {
        return Err(refusal(
            "SOURCE_CONTROL_STATE_CHANGED",
            "the review did not capture a stable snapshot; refresh and retry".to_string(),
        ));
    }
    if capture.status.snapshot_id != expected_snapshot_id {
        return Err(refusal(
            "SOURCE_CONTROL_STATE_CHANGED",
            "the repository changed since it was reviewed: branch, HEAD, index, or merge state"
                .to_string(),
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
        Ok(_) => finalize_commit(repo_root, &capture.status).await,
        Err(error) => {
            recover_if_head_moved(
                repo_root,
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
/// there the same finalization the success path takes reports what the commit
/// actually carries.
async fn recover_if_head_moved(
    repo_root: &Path,
    reviewed: &SourceControlStatus,
    error: AgentError,
) -> Result<CommitOutcome, AgentError> {
    if error.effect() == Some(MutationEffect::NotApplied.as_str()) {
        return Err(error);
    }
    match head_sha(repo_root).await {
        Ok(after) if after.is_some() && after != reviewed.head_sha => {
            finalize_commit(repo_root, reviewed).await
        }
        _ => Err(error),
    }
}

/// The current HEAD commit, or `None` on an unborn branch.
async fn head_sha(repo_root: &Path) -> Result<Option<String>, AgentError> {
    let call = GitCall::new(["rev-parse", "-q", "--verify", "HEAD"]).accept_exit_codes(&[1]);
    let output = runner::run_read(repo_root, call, None).await?;
    let sha = String::from_utf8_lossy(&trim_line_ending(output.stdout))
        .trim()
        .to_string();
    Ok((!sha.is_empty()).then_some(sha))
}
