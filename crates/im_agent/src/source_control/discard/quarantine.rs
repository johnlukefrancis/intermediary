// Path: crates/im_agent/src/source_control/discard/quarantine.rs
// Description: Quarantine directory naming and phase files for a discard operation, and the bounded startup sweep

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::source_control::locks::SourceControlLocks;

const QUARANTINE_DIR_NAME: &str = "intermediary-discard";
const CLAIMED_FILE_NAME: &str = "claimed";
const VERIFIED_FILE_NAME: &str = "verified";
const RETAINED_FILE_NAME: &str = "retained";
const UNRESTORED_FILE_NAME: &str = "unrestored";

static OP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// An operation id unique to one discard action: two discard calls racing on
/// the same repository (a retried request, or two clients) can never collide
/// their claims.
pub(super) fn generate_op_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let sequence = OP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{nanos:x}-{sequence:x}")
}

/// One directory per target, not per action: the phase files below are fixed
/// names, so two claiming targets sharing a directory would have the second's
/// retention replace the first's bytes and overwrite the marker that says what
/// they were. `<opId>-<targetIndex>` keeps every phase rule and the sweep
/// exactly as they are, one directory at a time, and still says which action
/// each directory belongs to.
pub(super) fn quarantine_root(git_dir: &Path, op_id: &str, target_index: usize) -> PathBuf {
    git_dir
        .join(QUARANTINE_DIR_NAME)
        .join(format!("{op_id}-{target_index}"))
}

/// The file a target is renamed into the moment the discard takes it out of
/// the worktree, before anything has been checked about it: `claimed` means
/// "moved here, nothing proven yet". One directory holds exactly one target,
/// so this name is never reused within it.
pub(super) fn claimed_file(quarantine_root: &Path) -> PathBuf {
    quarantine_root.join(CLAIMED_FILE_NAME)
}

/// Records that the claimed bytes matched the stamp the user reviewed and what
/// is about to happen at the original path. Written after the stamp check and
/// before the Git work, so a process that dies in between leaves a directory
/// that says for itself the destruction was authorized — and one that dies
/// before this leaves a directory that does not.
///
/// The directory carries exactly one marker, for the one target it holds.
pub(super) fn write_verified_marker(
    quarantine_root: &Path,
    path: &str,
    plan: &str,
) -> std::io::Result<()> {
    std::fs::write(
        quarantine_root.join(VERIFIED_FILE_NAME),
        format!("{path}\n{plan}\n"),
    )
}

/// The Git restore or removal landed, so the claim is superseded. The bytes
/// are kept as `retained` rather than deleted: they stay until the next agent
/// start, which is the whole window in which a user who discarded the wrong
/// thing can still be handed them back. A claim that is already gone leaves
/// nothing to retain and is not an error here.
pub(super) fn mark_retained(quarantine_root: &Path) -> std::io::Result<()> {
    let claimed = claimed_file(quarantine_root);
    match std::fs::rename(&claimed, quarantine_root.join(RETAINED_FILE_NAME)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// Renames a claim that could not be put back where it came from, taking it
/// out of the sweep's reach: the sweep finishes destructions that a `verified`
/// marker authorized, and these bytes are the opposite — the only copy of what
/// the user reviewed, with nothing at the original path. Best effort, because
/// there is nothing better to do if even this fails: the returned path is
/// wherever the bytes actually ended up, so the failure that follows can name
/// it.
pub(super) fn hold_unrestored(quarantined: &Path) -> PathBuf {
    let Some(root) = quarantined.parent() else {
        return quarantined.to_path_buf();
    };
    let held = root.join(UNRESTORED_FILE_NAME);
    match std::fs::rename(quarantined, &held) {
        Ok(()) => held,
        Err(_) => quarantined.to_path_buf(),
    }
}

/// Every quarantine directory left under `.git/intermediary-discard/` the
/// first time this process reads this git dir's status. Each directory says
/// for itself what may be done with it:
///
/// - this process created it: it is left alone entirely. Quarantined bytes are
///   retained until the *next* agent start, so the process that made a
///   directory is never the process that releases it — whether its discard is
///   still writing, has just finished, or finished long before this sweep ran.
/// - an `unrestored` file: a rollback or put-back could not return those bytes
///   to the worktree. They were never authorized for destruction, so the
///   directory stands and is logged.
/// - a `verified` marker and no `unrestored` file: those bytes were matched
///   against the stamp the user reviewed before anything was touched, so
///   removing the directory finishes exactly the destruction the discard was
///   authorized to do. The marker's path and plan are logged with it. This is
///   also how a successful discard's own retained bytes are eventually
///   released — retention lasts until the next agent start, not forever.
/// - neither: an earlier process died between claiming a file and verifying
///   it. Nothing ever proved those bytes were the ones the user confirmed, so
///   they stand too.
///
/// Bounded to one directory listing; nothing here can spin, one directory's
/// failure never stops the rest, and a failure is logged, never fatal to the
/// status read that triggered it.
pub(in crate::source_control) async fn sweep_stale_quarantine(
    git_dir: &Path,
    locks: &SourceControlLocks,
) {
    let root = git_dir.join(QUARANTINE_DIR_NAME);
    let log_root = root.clone();
    let locks = locks.clone();
    match tokio::task::spawn_blocking(move || sweep_blocking(&root, &locks)).await {
        Ok(Ok(swept)) => log_swept(&log_root, &swept),
        Ok(Err(error)) => log_sweep_failed(&log_root, &error),
        Err(join_error) => log_sweep_failed(&log_root, &join_error),
    }
}

/// What the sweep did with one stale operation directory, and why.
enum SweptOp {
    Removed {
        operation: PathBuf,
        path: String,
        plan: String,
    },
    Held {
        operation: PathBuf,
        reason: &'static str,
    },
    Failed {
        operation: PathBuf,
        error: String,
    },
}

/// Only the listing itself is fatal here. One directory that cannot be read or
/// removed is that directory's own outcome: it is reported and the sweep moves
/// on, because this runs once per process and an undeletable leftover must not
/// keep every other finished discard's bytes on disk forever.
fn sweep_blocking(root: &Path, locks: &SourceControlLocks) -> std::io::Result<Vec<SweptOp>> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut swept = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                swept.push(SweptOp::Failed {
                    operation: root.to_path_buf(),
                    error: error.to_string(),
                });
                continue;
            }
        };
        let operation = entry.path();
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => swept.push(sweep_one(operation, locks)),
            Ok(_) => continue,
            Err(error) => swept.push(SweptOp::Failed {
                operation,
                error: error.to_string(),
            }),
        }
    }
    Ok(swept)
}

fn sweep_one(operation: PathBuf, locks: &SourceControlLocks) -> SweptOp {
    if locks_created_it(&operation, locks) {
        return SweptOp::Held {
            operation,
            reason: "this process created it, and retention lasts until the next start",
        };
    }
    if operation.join(UNRESTORED_FILE_NAME).exists() {
        return SweptOp::Held {
            operation,
            reason: "it holds content a rollback or put-back could not restore",
        };
    }
    let Some((path, plan)) = verified_marker(&operation) else {
        return SweptOp::Held {
            operation,
            reason: "nothing in it says its content was ever matched against the review",
        };
    };
    if let Err(error) = std::fs::remove_dir_all(&operation) {
        return SweptOp::Failed {
            operation,
            error: error.to_string(),
        };
    }
    SweptOp::Removed {
        operation,
        path,
        plan,
    }
}

/// Asked at the moment of the decision, not at the start of the sweep: a
/// discard records its operation before creating any directory, and the record
/// is never withdrawn, so a directory this listing can see was either created
/// by an already-recorded operation of this process or by an earlier one. The
/// answer here can therefore never be stale in the direction that deletes.
fn locks_created_it(operation: &Path, locks: &SourceControlLocks) -> bool {
    operation
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| locks.created_by_this_process(name))
}

/// The `verified` marker's two lines: the worktree path the claim came from
/// and the plan that was about to run at it. A marker that cannot be read or
/// does not carry both lines is treated as no marker at all — nothing legible
/// is authorizing this removal.
fn verified_marker(operation: &Path) -> Option<(String, String)> {
    let content = std::fs::read_to_string(operation.join(VERIFIED_FILE_NAME)).ok()?;
    let mut lines = content.lines();
    let path = lines.next()?.to_string();
    let plan = lines.next()?.to_string();
    Some((path, plan))
}

fn log_swept(root: &Path, swept: &[SweptOp]) {
    let mut removed = 0_usize;
    let mut held = 0_usize;
    let mut failed = 0_usize;
    for operation in swept {
        match operation {
            SweptOp::Removed {
                operation,
                path,
                plan,
            } => {
                removed += 1;
                eprintln!(
                    "{{\"level\":\"info\",\"msg\":\"removed a finished source control discard quarantine directory\",\"operation\":{:?},\"path\":{path:?},\"plan\":{plan:?}}}",
                    operation.display().to_string()
                );
            }
            SweptOp::Held { operation, reason } => {
                held += 1;
                eprintln!(
                    "{{\"level\":\"warn\",\"msg\":\"kept a source control discard quarantine directory\",\"operation\":{:?},\"reason\":{reason:?}}}",
                    operation.display().to_string()
                );
            }
            SweptOp::Failed { operation, error } => {
                failed += 1;
                eprintln!(
                    "{{\"level\":\"warn\",\"msg\":\"could not sweep a source control discard quarantine directory\",\"operation\":{:?},\"error\":{error:?}}}",
                    operation.display().to_string()
                );
            }
        }
    }
    eprintln!(
        "{{\"level\":\"info\",\"msg\":\"source control discard quarantine sweep finished\",\"root\":{:?},\"removed\":{removed},\"held\":{held},\"failed\":{failed}}}",
        root.display().to_string()
    );
}

fn log_sweep_failed(root: &Path, error: &impl std::fmt::Display) {
    eprintln!(
        "{{\"level\":\"warn\",\"msg\":\"source control discard quarantine sweep failed\",\"root\":{:?},\"error\":{:?}}}",
        root.display().to_string(),
        error.to_string()
    );
}
