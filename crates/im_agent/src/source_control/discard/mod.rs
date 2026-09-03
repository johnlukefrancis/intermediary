// Path: crates/im_agent/src/source_control/discard/mod.rs
// Description: Discard exactly the confirmed targets, one at a time, under an operation-owned quarantine

mod claim;
pub(in crate::source_control) mod quarantine;
mod target;
#[cfg(test)]
mod tests_quarantine;
#[cfg(test)]
mod tests_stamps;
#[cfg(test)]
mod tests_sweep;

use std::collections::HashMap;
use std::path::Path;

use crate::error::{AgentError, MutationEffect};
use crate::protocol::{SourceControlChange, SourceControlDiscardTarget, SourceControlEntry, SourceControlStatus};

use self::quarantine::{generate_op_id, quarantine_root};
use self::target::{process_target, TargetError};
use crate::source_control::locks::SourceControlLocks;
use crate::source_control::paths::normalize_path;
use crate::source_control::runner;
use crate::source_control::status;

/// Only the confirmed targets are touched, one at a time: each is claimed into
/// its own quarantine directory before anything about it changes,
/// verified there against the stamp (or absence) the user reviewed, marked
/// with what that verification authorized, and only then acted on — so a
/// discard can never destroy a file that changed after the review, and a
/// crash mid-target leaves every earlier target's effect exactly as it
/// landed. The quarantined bytes outlive the action: they are kept until the
/// next agent start, so a user who discarded the wrong thing has somewhere to
/// go and get them. A copy row names its destination alone, and nothing
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
    let git_dir = runner::capture_location(repo_root, None)
        .await
        .map_err(|error| error.with_effect(MutationEffect::NotApplied))?
        .git_dir;
    let op_id = generate_op_id();
    // Registered before this action's first quarantine directory exists and
    // released when this future ends, however it ends. The startup sweep a
    // sibling configured root over the same git dir can fire at any moment
    // then leaves every directory this operation owns alone.
    let _live = locks.register_discard_op(&op_id);

    let mut applied: Vec<String> = Vec::new();
    for (index, target) in targets.iter().enumerate() {
        let root = quarantine_root(&git_dir, &op_id, index);
        match process_target(repo_root, &root, &classification, target).await {
            Ok(()) => applied.push(target.path.clone()),
            Err(error) => {
                cleanup_best_effort(&root).await;
                return Err(finish(error, &applied));
            }
        }
    }
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

/// A refusal leaves this target's own directory empty whenever the target
/// never got as far as a claim, and removing it then is tidiness, not
/// correctness. A directory that holds anything — retained bytes, a claim, an
/// unrestored hold — makes this removal simply fail, which is the point: that
/// content stands until the next process start sweeps what it is allowed to.
/// Earlier targets' directories are untouched either way.
async fn cleanup_best_effort(root: &Path) {
    let root = root.to_path_buf();
    let _ = tokio::task::spawn_blocking(move || std::fs::remove_dir(&root)).await;
}
