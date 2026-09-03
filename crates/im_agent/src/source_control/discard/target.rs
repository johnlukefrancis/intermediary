// Path: crates/im_agent/src/source_control/discard/target.rs
// Description: Executes one discard target: claim, classify, mutate, and release/rollback the claim

use std::collections::HashMap;
use std::path::Path;

use crate::error::{AgentError, MutationEffect};
use crate::protocol::{SourceControlChange, SourceControlDiscardTarget};

use super::claim::{claim_existing, Claim, ClaimFailure, ClaimOutcome};
use super::quarantine::{claimed_file, write_verified_marker};
use crate::source_control::paths::{nul_joined, PATHSPEC_FROM_STDIN};
use crate::source_control::runner::{self, GitCall, INDEX_TIMEOUT};
use crate::source_control::status::stamp::stamp_of;

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
            let claim = verified(repo_root, &target.path, claim, "restore").await?;
            settle(claim, restore_worktree(repo_root, &target.path).await).await
        }
        (Plan::RemoveIntentToAdd, ClaimOutcome::Claimed(claim)) => {
            let claim = verified(repo_root, &target.path, claim, "remove-intent-to-add").await?;
            settle(claim, unstage(repo_root, &target.path).await).await
        }
        (Plan::RemoveUntracked, ClaimOutcome::Claimed(claim)) => {
            let claim = verified(repo_root, &target.path, claim, "remove-untracked").await?;
            settle(claim, Ok(())).await
        }
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
        let repo_root = repo_root.to_path_buf();
        let path = target.path.clone();
        return blocking_claim(move || {
            verify_absent_without_stamp(&repo_root, &path).map(|()| ClaimOutcome::Nothing)
        })
        .await;
    };
    let repo_root = repo_root.to_path_buf();
    let quarantine_root = quarantine_root.to_path_buf();
    let path = target.path.clone();
    blocking_claim(move || {
        claim_existing(&repo_root, &quarantine_root, &path, expected).map(ClaimOutcome::Claimed)
    })
    .await
}

/// The file must still be absent from `path`: reviewed as `worktreeMissing`,
/// it is refused instead of restored-over if a newer file has since appeared.
fn verify_still_missing(repo_root: &Path, path: &str) -> Result<(), ClaimFailure> {
    if stamp_of(&repo_root.join(path)).missing {
        return Ok(());
    }
    Err(ClaimFailure::Refused(AgentError::new(
        "SOURCE_CONTROL_STATE_CHANGED",
        format!("{path} was created after it was reviewed"),
    )))
}

/// The review asserted nothing at all about this path's bytes. That is the
/// rename origin the UI sends — a path it showed as already gone — and it is
/// only ever restored, never removed, so an absent path is exactly what this
/// target should be. A path that is there is a different thing entirely:
/// whatever it is (a directory, a symlink, a file this process could not stat
/// through an unreadable parent), the review never stamped it, so nothing can
/// prove a discard would destroy what the user actually looked at.
fn verify_absent_without_stamp(repo_root: &Path, path: &str) -> Result<(), ClaimFailure> {
    if stamp_of(&repo_root.join(path)).missing {
        return Ok(());
    }
    Err(ClaimFailure::Refused(AgentError::new(
        "SOURCE_CONTROL_STATE_CHANGED",
        format!(
            "cannot identify {path} before discarding it (not a regular file the review could stamp)"
        ),
    )))
}

/// Records what this claim was matched against and what is about to happen at
/// the original path, before any of it happens. A marker that cannot be
/// written is a refusal rather than a discard: the claimed file goes straight
/// back where it came from and nothing else runs, because a process that then
/// died would leave bytes in quarantine that no later sweep could tell from
/// ones nobody ever checked.
async fn verified(
    repo_root: &Path,
    path: &str,
    claim: Claim,
    plan: &'static str,
) -> Result<Claim, TargetError> {
    let root = claim.root.clone();
    let recorded = path.to_string();
    let written = tokio::task::spawn_blocking(move || write_verified_marker(&root, &recorded, plan));
    let reason = match written.await {
        Ok(Ok(())) => return Ok(claim),
        Ok(Err(error)) => format!("Could not record the discard of {path} before running it: {error}"),
        Err(join_error) => format!("Discard task failed: {join_error}"),
    };
    put_back(repo_root, path, claim).await?;
    Err(TargetError::Refused(AgentError::internal(reason)))
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

/// The tail every verified target shares. From the moment the marker is
/// written the worktree path is empty, the claimed bytes are the only copy of
/// what the user reviewed, and that marker authorizes the next start's sweep
/// to delete them — so the claim is always resolved here before anything
/// returns. The plan's work landing retains the bytes; anything else, the Git
/// command's failure or the retention's own, holds them out of the sweep's
/// reach and reports where they are instead of leaving a failure that never
/// names them.
async fn settle(claim: Claim, acted: Result<(), TargetError>) -> Result<(), TargetError> {
    let reason = match acted {
        Ok(()) => None,
        Err(TargetError::Refused(error) | TargetError::EffectUnknown(error)) => {
            Some(error.message().to_string())
        }
    };
    let claimed = claimed_file(&claim.root);
    let resolved = tokio::task::spawn_blocking(move || match reason {
        Some(reason) => Err(claim.hold(reason)),
        None => claim.release(),
    })
    .await;
    match resolved {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(TargetError::EffectUnknown(error)),
        // Nothing ran at all, so the bytes are still under the name the sweep
        // would finish: name them where they actually are.
        Err(join_error) => Err(TargetError::EffectUnknown(
            AgentError::internal(format!(
                "Discard task failed: {join_error}; the reviewed bytes are at {}",
                claimed.display()
            ))
            .with_effect(MutationEffect::Unknown),
        )),
    }
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
