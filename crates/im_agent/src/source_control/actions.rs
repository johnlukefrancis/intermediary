// Path: crates/im_agent/src/source_control/actions.rs
// Description: Dispatches one source-control mutation and reads the status that follows it

use std::path::Path;

use crate::error::{AgentError, MutationEffect};
use crate::protocol::{SourceControlActionKind, SourceControlActionPayload};

use super::actions_commit::commit;
use super::actions_discard::discard;
use super::actions_remote::{pull, push};
use super::actions_stage::{stage, unstage};
use super::locks::SourceControlLocks;
use super::{status, SourceControlActionOutcome};

/// Runs the mutation, then the fresh status every kind reads afterwards. A
/// commit already carries its own new HEAD and any hook-changed paths from
/// `actions_commit::commit`, which itself proves the commit landed before
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
    let (commit_sha, hook_changed_paths) = match action {
        SourceControlActionPayload::Stage { scope } => {
            stage(repo_root, scope, locks).await?;
            (None, Vec::new())
        }
        SourceControlActionPayload::Unstage { scope } => {
            unstage(repo_root, scope, locks).await?;
            (None, Vec::new())
        }
        SourceControlActionPayload::Discard { targets } => {
            discard(repo_root, &targets, locks).await?;
            (None, Vec::new())
        }
        SourceControlActionPayload::Commit {
            message,
            expected_index_tree_sha,
            expected_head_sha,
        } => {
            let outcome = commit(
                repo_root,
                &message,
                &expected_index_tree_sha,
                expected_head_sha.as_deref(),
                locks,
            )
            .await?;
            (Some(outcome.commit_sha), outcome.hook_changed_paths)
        }
        SourceControlActionPayload::Push => {
            push(repo_root, locks).await?;
            (None, Vec::new())
        }
        SourceControlActionPayload::Pull => {
            pull(repo_root).await?;
            (None, Vec::new())
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
