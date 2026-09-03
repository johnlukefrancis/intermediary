// Path: crates/im_agent/src/source_control/actions/mod.rs
// Description: Dispatches one source-control mutation and reads the status that follows it

mod remote;
mod stage;
#[cfg(test)]
mod tests;

use std::path::Path;

use crate::error::{AgentError, MutationEffect};
use crate::protocol::{SourceControlActionKind, SourceControlActionPayload};

use self::remote::{pull, push};
use self::stage::{stage, unstage};
use crate::source_control::commit::commit;
use crate::source_control::discard::discard;
use crate::source_control::locks::SourceControlLocks;
use crate::source_control::{status, SourceControlActionOutcome};

/// Runs the mutation, then the fresh status every kind reads afterwards. A
/// commit already carries its own new HEAD and whatever a hook did to it from
/// `commit::commit`, which itself proves the commit landed before
/// returning `Ok`. Once the mutation has landed, a failing follow-up status
/// read never surfaces as a `GIT_*` error; it is reported as an
/// unknown-outcome error so the UI reconciles by refetching instead of telling
/// the user the action failed.
pub(super) async fn run_action(
    repo_root: &Path,
    action: SourceControlActionPayload,
    locks: &SourceControlLocks,
) -> Result<SourceControlActionOutcome, AgentError> {
    let kind = action.kind();
    let (commit_sha, hook_changed_paths, hook_added_paths) = match action {
        SourceControlActionPayload::Stage { scope } => {
            stage(repo_root, scope, locks).await?;
            (None, None, None)
        }
        SourceControlActionPayload::Unstage { scope } => {
            unstage(repo_root, scope, locks).await?;
            (None, None, None)
        }
        SourceControlActionPayload::Discard { targets } => {
            discard(repo_root, &targets, locks).await?;
            (None, None, None)
        }
        SourceControlActionPayload::Commit {
            message,
            expected_snapshot_id,
        } => {
            let outcome = commit(repo_root, &message, &expected_snapshot_id, locks).await?;
            (
                Some(outcome.commit_sha),
                outcome.hook_changed_paths,
                outcome.hook_added_paths,
            )
        }
        SourceControlActionPayload::Push => {
            push(repo_root, locks).await?;
            (None, None, None)
        }
        SourceControlActionPayload::Pull => {
            pull(repo_root).await?;
            (None, None, None)
        }
    };
    let status = status::capture_status(repo_root, None, locks)
        .await
        .map_err(|error| applied_but_unread(kind, commit_sha.clone(), error))?
        .status;
    Ok(SourceControlActionOutcome {
        status,
        commit_sha,
        hook_changed_paths,
        hook_added_paths,
    })
}

/// The mutation landed and the read after it did not. The effect is unknown to
/// the wire contract's vocabulary — applied, but with no state to show — so the
/// UI reconciles rather than reporting a failure.
fn applied_but_unread(
    kind: SourceControlActionKind,
    commit_sha: Option<String>,
    inner: AgentError,
) -> AgentError {
    let name = format!("{kind:?}").to_ascii_lowercase();
    AgentError::new(
        "ACTION_APPLIED_STATUS_UNAVAILABLE",
        format!(
            "{name} completed but the follow-up status read failed: {}",
            inner.message()
        ),
    )
    .with_details(serde_json::json!({ "kind": kind, "commitSha": commit_sha }))
    .with_effect(MutationEffect::Unknown)
}
