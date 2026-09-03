// Path: crates/im_agent/src/source_control/discard_quarantine.rs
// Description: Quarantine directory naming for a discard operation and the bounded startup sweep of stale ones

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const QUARANTINE_DIR_NAME: &str = "intermediary-discard";
const CLAIMED_FILE_NAME: &str = "claimed";
const UNRESTORED_FILE_NAME: &str = "unrestored";

static OP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// A quarantine directory unique to one discard action: two discard calls
/// racing on the same repository (a retried request, or two clients) can
/// never collide their claims.
pub(super) fn generate_op_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let sequence = OP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{nanos:x}-{sequence:x}")
}

pub(super) fn quarantine_root(git_dir: &Path, op_id: &str) -> PathBuf {
    git_dir.join(QUARANTINE_DIR_NAME).join(op_id)
}

/// Targets are processed one at a time, so exactly one claim is ever live
/// under one operation's directory: it always claims to the same filename.
pub(super) fn claimed_file(quarantine_root: &Path) -> PathBuf {
    quarantine_root.join(CLAIMED_FILE_NAME)
}

/// Renames a claim that could not be put back where it came from, taking it
/// out of the sweep's reach: `claimed` means "verified against the review and
/// authorized for destruction", and these bytes are neither — they are the
/// only copy of what the user reviewed. Best effort, because there is nothing
/// better to do if even this fails: the returned path is wherever the bytes
/// actually ended up, so the failure that follows can name it.
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

/// Every `<opId>` directory left under `.git/intermediary-discard/` the first
/// time this process reads this git dir's status: an earlier agent process
/// claimed a file into it and never finished (crash, forced stop) — the
/// operation that owned it is gone, and every target it might still hold was
/// already claimed only after being verified byte-for-byte against what the
/// user reviewed, so removing it is exactly finishing the destruction the
/// discard had already been authorized to do.
///
/// The one exception is a directory holding an `unrestored` file: those bytes
/// were never authorized for destruction — a rollback failed to put them back
/// — so the directory is left standing and logged instead. Bounded to one
/// directory listing; nothing here can spin, and a failure is logged, never
/// fatal to the status read that triggered it.
pub(super) async fn sweep_stale_quarantine(git_dir: &Path) {
    let root = git_dir.join(QUARANTINE_DIR_NAME);
    let log_root = root.clone();
    match tokio::task::spawn_blocking(move || sweep_blocking(&root)).await {
        Ok(Ok(swept)) => log_swept(&log_root, &swept),
        Ok(Err(error)) => log_sweep_failed(&log_root, &error),
        Err(join_error) => log_sweep_failed(&log_root, &join_error),
    }
}

/// What one sweep did: how many stale operation directories it removed, and
/// the ones it deliberately left standing because they still hold reviewed
/// bytes no rollback managed to put back.
struct SweptQuarantine {
    removed: u32,
    held: Vec<PathBuf>,
}

fn sweep_blocking(root: &Path) -> std::io::Result<SweptQuarantine> {
    let mut swept = SweptQuarantine {
        removed: 0,
        held: Vec::new(),
    };
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(swept),
        Err(error) => return Err(error),
    };
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let operation = entry.path();
        if operation.join(UNRESTORED_FILE_NAME).exists() {
            swept.held.push(operation);
            continue;
        }
        std::fs::remove_dir_all(&operation)?;
        swept.removed += 1;
    }
    Ok(swept)
}

fn log_swept(root: &Path, swept: &SweptQuarantine) {
    if swept.removed > 0 {
        eprintln!(
            "{{\"level\":\"info\",\"msg\":\"removed stale source control discard quarantine directories\",\"root\":{:?},\"removed\":{}}}",
            root.display().to_string(),
            swept.removed
        );
    }
    for operation in &swept.held {
        eprintln!(
            "{{\"level\":\"warn\",\"msg\":\"kept a discard quarantine directory holding content a rollback could not restore\",\"operation\":{:?}}}",
            operation.display().to_string()
        );
    }
}

fn log_sweep_failed(root: &Path, error: &impl std::fmt::Display) {
    eprintln!(
        "{{\"level\":\"warn\",\"msg\":\"source control discard quarantine sweep failed\",\"root\":{:?},\"error\":{:?}}}",
        root.display().to_string(),
        error.to_string()
    );
}
