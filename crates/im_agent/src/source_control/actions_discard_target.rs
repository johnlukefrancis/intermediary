// Path: crates/im_agent/src/source_control/actions_discard_target.rs
// Description: Executes one discard target: claim, classify, mutate, and release/rollback the claim

use std::collections::HashMap;
use std::path::Path;

use crate::error::{AgentError, MutationEffect};
use crate::protocol::{SourceControlChange, SourceControlDiscardTarget};

use super::actions_discard_claim::{
    claim_existing, verify_still_missing, Claim, ClaimFailure, ClaimOutcome,
};
use super::paths::{nul_joined, PATHSPEC_FROM_STDIN};
use super::runner::{self, GitCall, INDEX_TIMEOUT};

/// What is done at the original path once the target has been claimed (or
/// confirmed to need no claim).
enum Plan {
    NoOp,
    Restore,
    RemoveUntracked,
    RemoveIntentToAdd,
}

/// A refusal proven safe for this target (nothing about its file changed,
/// because a rollback already ran) versus a failure that leaves this target's
/// state unproven — the caller can no longer default the whole action's
/// effect to `notApplied` once this happens.
pub(super) enum TargetError {
    Refused(AgentError),
    EffectUnknown(AgentError),
}

/// One target, start to finish, under the caller's mutation lock.
pub(super) async fn process_target(
    repo_root: &Path,
    quarantine_root: &Path,
    classification: &HashMap<&str, SourceControlChange>,
    target: &SourceControlDiscardTarget,
) -> Result<(), TargetError> {
    let claim = claim(repo_root, quarantine_root, target).await?;
    let plan = plan_for(classification.get(target.path.as_str()), &claim);
    match (plan, claim) {
        (Plan::NoOp, ClaimOutcome::Nothing) => Ok(()),
        (Plan::NoOp, ClaimOutcome::Claimed(claim)) => put_back(repo_root, &target.path, claim).await,
        (Plan::Restore, ClaimOutcome::Nothing) => restore_worktree(repo_root, &target.path).await,
        (Plan::Restore, ClaimOutcome::Claimed(claim)) => {
            restore_worktree(repo_root, &target.path).await?;
            release(claim).await
        }
        (Plan::RemoveIntentToAdd, ClaimOutcome::Claimed(claim)) => {
            unstage(repo_root, &target.path).await?;
            release(claim).await
        }
        (Plan::RemoveUntracked, ClaimOutcome::Claimed(claim)) => release(claim).await,
        // A removal plan is only ever reached when the target was confirmed on
        // disk, i.e. via a successful claim; this pairing cannot occur.
        (Plan::RemoveIntentToAdd | Plan::RemoveUntracked, ClaimOutcome::Nothing) => Ok(()),
    }
}

async fn claim(
    repo_root: &Path,
    quarantine_root: &Path,
    target: &SourceControlDiscardTarget,
) -> Result<ClaimOutcome, TargetError> {
    if target.expected_missing {
        let repo_root = repo_root.to_path_buf();
        let path = target.path.clone();
        return blocking_claim(move || {
            verify_still_missing(&repo_root, &path).map(|()| ClaimOutcome::Nothing)
        })
        .await;
    }
    let Some(expected) = target.expected_stamp else {
        return Ok(ClaimOutcome::Nothing);
    };
    let repo_root = repo_root.to_path_buf();
    let quarantine_root = quarantine_root.to_path_buf();
    let path = target.path.clone();
    blocking_claim(move || {
        claim_existing(&repo_root, &quarantine_root, &path, expected).map(ClaimOutcome::Claimed)
    })
    .await
}

/// `Untracked`/`Added` (intent-to-add) only resolve to a removal when the
/// claim confirmed the file is actually on disk; a target reviewed as missing
/// (no claim made) leaves those sections alone entirely — nothing in the
/// index or worktree there for a removal to act on.
fn plan_for(change: Option<&SourceControlChange>, claim: &ClaimOutcome) -> Plan {
    let on_disk = matches!(claim, ClaimOutcome::Claimed(_));
    match change {
        Some(SourceControlChange::Untracked) if on_disk => Plan::RemoveUntracked,
        Some(SourceControlChange::Untracked) => Plan::NoOp,
        Some(SourceControlChange::Added) if on_disk => Plan::RemoveIntentToAdd,
        Some(_) => Plan::Restore,
        None => Plan::NoOp,
    }
}

async fn restore_worktree(repo_root: &Path, path: &str) -> Result<(), TargetError> {
    run_git(repo_root, GitCall::new(["restore", "--worktree"]), path).await
}

async fn unstage(repo_root: &Path, path: &str) -> Result<(), TargetError> {
    run_git(repo_root, GitCall::new(["reset", "-q"]), path).await
}

/// A Git failure here is never provably safe: a claimed target is already
/// missing from its original location until this command (or the release
/// that follows it) finishes, and an unclaimed `Restore` is this target's only
/// effect boundary. Either way the effect is forced `unknown`, overriding any
/// `notApplied` the Git layer itself proved (a missing executable proves the
/// command never ran, not that this target's file is still where it was).
async fn run_git(repo_root: &Path, call: GitCall, path: &str) -> Result<(), TargetError> {
    let call = call
        .args(PATHSPEC_FROM_STDIN)
        .stdin(nul_joined(&[path.to_string()]))
        .timeout(INDEX_TIMEOUT);
    runner::run_mutation(repo_root, call)
        .await
        .map(drop)
        .map_err(|error| TargetError::EffectUnknown(error.with_effect(MutationEffect::Unknown)))
}

async fn release(claim: Claim) -> Result<(), TargetError> {
    blocking_io(move || claim.release()).await
}

/// The one cleanup that can strand the bytes the user reviewed: its failure
/// already carries the message naming where they are held, so it is passed
/// through as-is rather than wrapped in the generic cleanup wording.
async fn put_back(repo_root: &Path, path: &str, claim: Claim) -> Result<(), TargetError> {
    let repo_root = repo_root.to_path_buf();
    let path = path.to_string();
    match tokio::task::spawn_blocking(move || claim.restore(&repo_root, &path)).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(TargetError::EffectUnknown(error)),
        Err(join_error) => Err(TargetError::EffectUnknown(AgentError::internal(format!(
            "Discard task failed: {join_error}"
        )))),
    }
}

async fn blocking_claim<T, F>(work: F) -> Result<T, TargetError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, ClaimFailure> + Send + 'static,
{
    match tokio::task::spawn_blocking(work).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(ClaimFailure::Refused(error))) => Err(TargetError::Refused(error)),
        Ok(Err(ClaimFailure::EffectUnknown(error))) => Err(TargetError::EffectUnknown(error)),
        Err(join_error) => Err(TargetError::EffectUnknown(AgentError::internal(format!(
            "Discard task failed: {join_error}"
        )))),
    }
}

async fn blocking_io<T, F>(work: F) -> Result<T, TargetError>
where
    T: Send + 'static,
    F: FnOnce() -> std::io::Result<T> + Send + 'static,
{
    match tokio::task::spawn_blocking(work).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(TargetError::EffectUnknown(
            AgentError::internal(format!("Discard cleanup failed: {error}"))
                .with_effect(MutationEffect::Unknown),
        )),
        Err(join_error) => Err(TargetError::EffectUnknown(AgentError::internal(format!(
            "Discard task failed: {join_error}"
        )))),
    }
}
