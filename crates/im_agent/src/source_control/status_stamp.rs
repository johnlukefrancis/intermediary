// Path: crates/im_agent/src/source_control/status_stamp.rs
// Description: Size/mtime/presence reads for worktree and conflict entries, and the shared stamp reader

use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::error::AgentError;
use crate::protocol::{SourceControlStatus, SourceControlWorktreeStamp};

/// What `fs::symlink_metadata` says about one status entry's path right now: a
/// regular file (with the stamp a discard later verifies against), something
/// else that exists (a directory or symlink — never stamped, never treated as
/// missing either), or genuinely absent, which the wire reports as
/// `worktreeMissing` so a discard can tell "deleted at review" from "still
/// there but unreadable".
///
/// Only `NotFound` is absence. A path this process cannot stat for any other
/// reason (a permission denial on a parent directory, an unreadable mount) is
/// neither stamped nor called missing: a discard then has no assertion to
/// verify and restores rather than removes, instead of claiming a file it never
/// managed to look at is gone.
pub(super) struct DiskState {
    pub stamp: Option<SourceControlWorktreeStamp>,
    pub missing: bool,
}

pub(super) fn stamp_of(path: &Path) -> DiskState {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => DiskState {
            stamp: mtime_parts(&metadata).map(|(mtime_ms, mtime_nanos)| SourceControlWorktreeStamp {
                bytes: metadata.len(),
                mtime_ms,
                mtime_nanos,
            }),
            missing: false,
        },
        Ok(_) => DiskState {
            stamp: None,
            missing: false,
        },
        Err(error) => DiskState {
            stamp: None,
            missing: error.kind() == std::io::ErrorKind::NotFound,
        },
    }
}

/// Milliseconds since the epoch (matching what a browser reports) alongside
/// the nanosecond-of-second remainder `fs::metadata` actually carries, so a
/// same-length rewrite that lands in the same millisecond is still caught by
/// a discard's stamp comparison. Before-epoch mtimes are the rare case of a
/// deliberately backdated file; the magnitude is preserved and negated the
/// same way the millisecond field already was, so equality still holds
/// between two reads of the same unmoved file.
fn mtime_parts(metadata: &std::fs::Metadata) -> Option<(i64, u32)> {
    let modified = metadata.modified().ok()?;
    match modified.duration_since(UNIX_EPOCH) {
        Ok(since_epoch) => Some((
            i64::try_from(since_epoch.as_millis()).ok()?,
            since_epoch.subsec_nanos(),
        )),
        Err(before_epoch) => {
            let duration = before_epoch.duration();
            Some((-i64::try_from(duration.as_millis()).ok()?, duration.subsec_nanos()))
        }
    }
}

/// Fills the stamp and presence of every worktree and conflict entry. Index
/// entries keep neither: their content is in the index, and the file on disk
/// is a different object. The metadata pass is filesystem work and runs off
/// the runtime (ADR-009).
pub(super) async fn stamp_worktree_entries(
    repo_root: &Path,
    mut status: SourceControlStatus,
) -> Result<SourceControlStatus, AgentError> {
    let repo_root = repo_root.to_path_buf();
    tokio::task::spawn_blocking(move || {
        for entry in status.worktree.iter_mut().chain(status.conflicts.iter_mut()) {
            let disk = stamp_of(&repo_root.join(&entry.path));
            entry.worktree_stamp = disk.stamp;
            entry.worktree_missing = disk.missing;
        }
        status
    })
    .await
    .map_err(|error| AgentError::internal(format!("Stamp task failed: {error}")))
}
