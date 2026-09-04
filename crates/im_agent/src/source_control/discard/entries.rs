// Path: crates/im_agent/src/source_control/discard/entries.rs
// Description: Removing chosen worktree entries by claiming each into this repository's discard quarantine

//! Deleting an entry from the worktree is a claim, not an unlink: the entry is
//! renamed into a per-operation quarantine directory beside a marker saying
//! what it was and why it left, and it stays there until the next agent start
//! sweeps it. A user who deleted the wrong folder therefore has somewhere to
//! go and get it, and the sweep's whitelist — remove only what a legible
//! `verified` marker authorized — governs these directories exactly as it
//! governs a discard's.
//!
//! The claim itself is a plain `std::fs::rename` into this operation's own
//! empty slot, which is what makes a whole directory removable in one
//! operation. Like `claim.rs`, the calls here are synchronous `std::fs` from
//! an async fn on purpose: a rename inside one filesystem and two small marker
//! writes are microseconds, and moving them to a blocking pool would buy
//! nothing but a task hop per entry.

use std::io;
use std::path::Path;

use crate::error::{AgentError, MutationEffect};
use crate::source_control::locks::SourceControlLocks;
use crate::source_control::paths::{ensure_no_git_component, ensure_within_root, normalize_path};
use crate::source_control::runner;

use super::cleanup_best_effort;
use super::quarantine::{
    claimed_file, generate_op_id, mark_retained, quarantine_root, write_verified_marker,
};

/// What the marker beside these bytes says was about to happen at the original
/// path. The startup sweep logs it, so a released directory says for itself
/// that a delete — not a discard's restore or removal — is what it finished.
const DELETE_PLAN: &str = "delete";

/// How one entry's claim failed. `Refused` proves that entry never left the
/// worktree; `Landed` says it did, and the action is half-applied from here.
enum EntryFailure {
    Refused(AgentError),
    Landed(AgentError),
}

/// Removes `paths` from the worktree, recoverably, and returns them in the
/// order given.
///
/// Every entry is validated — normalized, refused if it names the Git
/// directory, proven inside the root, proven to exist — before the first claim
/// moves anything, so a bad path in a selection of ten refuses all ten with
/// nothing touched. After the first entry has left the worktree, any failure
/// carries `details.applied` and an `unknown` effect: the action is
/// half-applied, and only a fresh read can say what the worktree holds now.
///
/// The caller must already hold this worktree's mutation lock.
pub(crate) async fn quarantine_entries(
    repo_root: &Path,
    paths: &[String],
    locks: &SourceControlLocks,
) -> Result<Vec<String>, AgentError> {
    if paths.is_empty() {
        return Err(AgentError::new("INVALID_PATH", "No paths given")
            .with_effect(MutationEffect::NotApplied));
    }
    // Everything up to the first claim proves itself: no entry has moved, so
    // each refusal here says so rather than leaning on an outer default.
    let planned = plan(repo_root, paths).map_err(not_applied)?;
    let git_dir = runner::capture_location(repo_root, None)
        .await
        .map_err(not_applied)?
        .git_dir;
    let op_id = generate_op_id();
    // Recorded before this action's first quarantine directory exists, and
    // never withdrawn: a directory this process created is never removed by
    // this process's sweep, so the deleted bytes stand until the next agent
    // start however this process's reads and mutations interleave.
    locks.register_discard_op(&op_id);

    let mut removed: Vec<String> = Vec::with_capacity(planned.len());
    for (index, path) in planned.iter().enumerate() {
        let root = quarantine_root(&git_dir, &op_id, index);
        match claim_entry(repo_root, &root, path) {
            Ok(()) => removed.push(path.clone()),
            Err(EntryFailure::Refused(error)) => {
                // Nothing moved for this entry, so its directory is empty
                // unless an earlier phase left content in it; removing it then
                // is tidiness, and a directory holding anything simply stands.
                cleanup_best_effort(&root).await;
                return Err(finish(error, &removed));
            }
            Err(EntryFailure::Landed(error)) => {
                removed.push(path.clone());
                return Err(finish(error, &removed));
            }
        }
    }
    Ok(removed)
}

fn not_applied(error: AgentError) -> AgentError {
    error.with_effect(MutationEffect::NotApplied)
}

fn plan(repo_root: &Path, paths: &[String]) -> Result<Vec<String>, AgentError> {
    let mut planned = Vec::with_capacity(paths.len());
    for path in paths {
        let normalized = normalize_path(path)?;
        ensure_no_git_component(&normalized)?;
        ensure_within_root(repo_root, &normalized)?;
        if std::fs::symlink_metadata(repo_root.join(&normalized)).is_err() {
            return Err(AgentError::new(
                "ENTRY_NOT_FOUND",
                format!("{normalized} no longer exists"),
            ));
        }
        planned.push(normalized);
    }
    Ok(planned)
}

/// One entry claimed out of the worktree and recorded as removed.
///
/// The order is the recovery contract: create the slot, take the entry into it
/// (this is the moment the worktree changes), record what was taken and why,
/// then retain the bytes under the name the sweep releases. A process that
/// dies before the marker leaves bytes nothing authorized destroying, and the
/// sweep keeps them; one that dies after leaves a directory that says for
/// itself the removal was asked for.
fn claim_entry(repo_root: &Path, root: &Path, path: &str) -> Result<(), EntryFailure> {
    std::fs::create_dir_all(root).map_err(|error| {
        EntryFailure::Refused(AgentError::internal(format!(
            "Could not prepare a quarantine directory for {path}: {error}"
        )))
    })?;
    if let Err(error) = std::fs::rename(repo_root.join(path), claimed_file(root)) {
        return Err(EntryFailure::Refused(unclaimable(path, &error)));
    }
    write_verified_marker(root, path, DELETE_PLAN).map_err(|error| {
        EntryFailure::Landed(AgentError::internal(format!(
            "Removed {path} but could not record it in quarantine: {error}"
        )))
    })?;
    mark_retained(root).map_err(|error| {
        EntryFailure::Landed(AgentError::internal(format!(
            "Removed {path} but could not retain its bytes: {error}"
        )))
    })
}

/// The claim's own rename failed, so this entry never moved. A worktree on a
/// different volume from its own repository can never be claimed at all: no
/// rename will move a file between them, and that is a layout the user has to
/// change, not a state that will settle. An entry that is simply gone by now
/// changed between the selection and this click, which is worth saying
/// plainly rather than reporting as an internal fault.
fn unclaimable(path: &str, error: &io::Error) -> AgentError {
    match error.kind() {
        io::ErrorKind::CrossesDevices => AgentError::new(
            "SOURCE_CONTROL_UNSUPPORTED_LAYOUT",
            format!(
                "Cannot remove {path}: the worktree and its repository live on different volumes"
            ),
        ),
        io::ErrorKind::NotFound => {
            AgentError::new("ENTRY_NOT_FOUND", format!("{path} no longer exists"))
        }
        _ => AgentError::internal(format!("Could not remove {path}: {error}")),
    }
}

/// A failure with no earlier removal in this action is exactly what it says:
/// nothing about the worktree changed. Once an entry has been claimed, the
/// same failure is no longer a clean refusal — the action is half-applied — so
/// the outcome is unknown and `details.applied` names what already left.
fn finish(error: AgentError, removed: &[String]) -> AgentError {
    if removed.is_empty() {
        return error.with_effect(MutationEffect::NotApplied);
    }
    error
        .with_details(serde_json::json!({ "applied": removed }))
        .with_effect(MutationEffect::Unknown)
}
