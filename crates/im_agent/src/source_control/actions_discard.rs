// Path: crates/im_agent/src/source_control/actions_discard.rs
// Description: Discard exactly the confirmed targets, one at a time, under an operation-owned quarantine

use std::collections::HashMap;
use std::path::Path;

use crate::error::{AgentError, MutationEffect};
use crate::protocol::{SourceControlChange, SourceControlDiscardTarget, SourceControlEntry, SourceControlStatus};

use super::actions_discard_target::{process_target, TargetError};
use super::discard_quarantine::{generate_op_id, quarantine_root};
use super::locks::SourceControlLocks;
use super::paths::normalize_path;
use super::runner;
use super::status;

/// Only the confirmed targets are touched, one at a time: each is claimed into
/// an operation-owned quarantine directory before anything about it changes,
/// verified there against the stamp (or absence) the user reviewed, and only
/// then acted on — so a discard can never destroy a file that changed after
/// the review, and a crash mid-target leaves every earlier target's effect
/// exactly as it landed. A copy row names its destination alone, and nothing
/// here expands a record's provenance into a second target, so discarding a
/// copy can never reach the source file the user was not shown.
pub(super) async fn discard(
    repo_root: &Path,
    targets: &[SourceControlDiscardTarget],
    locks: &SourceControlLocks,
) -> Result<(), AgentError> {
    if targets.is_empty() {
        return Err(
            AgentError::new("INVALID_PATH", "No paths given").with_effect(MutationEffect::NotApplied)
        );
    }
    let targets = normalize(targets)?;
    let status = status::capture_status(repo_root, None, locks)
        .await
        .map_err(|error| error.with_effect(MutationEffect::NotApplied))?
        .status;
    let classification = classify(&status);
    let git_dir = runner::capture_location(repo_root, None).await?.git_dir;
    let root = quarantine_root(&git_dir, &generate_op_id());

    let mut applied: Vec<String> = Vec::new();
    for target in &targets {
        match process_target(repo_root, &root, &classification, target).await {
            Ok(()) => applied.push(target.path.clone()),
            Err(error) => {
                cleanup_best_effort(&root).await;
                return Err(finish(error, &applied));
            }
        }
    }
    cleanup_best_effort(&root).await;
    Ok(())
}

fn normalize(
    targets: &[SourceControlDiscardTarget],
) -> Result<Vec<SourceControlDiscardTarget>, AgentError> {
    targets
        .iter()
        .map(|target| {
            Ok(SourceControlDiscardTarget {
                path: normalize_path(&target.path)
                    .map_err(|error| error.with_effect(MutationEffect::NotApplied))?,
                expected_stamp: target.expected_stamp,
                expected_missing: target.expected_missing,
            })
        })
        .collect()
}

fn classify(status: &SourceControlStatus) -> HashMap<&str, SourceControlChange> {
    status
        .worktree
        .iter()
        .chain(&status.conflicts)
        .map(|entry: &SourceControlEntry| (entry.path.as_str(), entry.change))
        .collect()
}

/// A failure with no earlier success in this action is exactly what it says:
/// nothing about the repository changed. Once an earlier target has landed,
/// the same failure is no longer a clean refusal — the action is now
/// half-applied — so the outcome is unknown, and the message names what
/// already happened so the user is not left guessing which targets survived.
fn finish(error: TargetError, applied: &[String]) -> AgentError {
    let has_prior_success = !applied.is_empty();
    let (code, message, unknown) = match error {
        TargetError::Refused(inner) => (
            inner.code().to_string(),
            inner.message().to_string(),
            has_prior_success,
        ),
        TargetError::EffectUnknown(inner) => (inner.code().to_string(), inner.message().to_string(), true),
    };
    let message = if has_prior_success {
        format!("{message}; already discarded: {}", applied.join(", "))
    } else {
        message
    };
    let effect = if unknown {
        MutationEffect::Unknown
    } else {
        MutationEffect::NotApplied
    };
    AgentError::new(code, message).with_effect(effect)
}

/// The operation directory should be empty by now (every claim was released
/// or rolled back); removing it is tidiness, not correctness — a directory
/// left behind by a failed removal is still swept at the next process start.
async fn cleanup_best_effort(root: &Path) {
    let root = root.to_path_buf();
    let _ = tokio::task::spawn_blocking(move || std::fs::remove_dir(&root)).await;
}
