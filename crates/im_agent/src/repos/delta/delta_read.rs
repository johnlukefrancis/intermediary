// Path: crates/im_agent/src/repos/delta/delta_read.rs
// Description: Blocking stat-read-restat of one settled file for the delta pipeline

use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::protocol::OpaqueReason;

use super::MAX_DELTA_FILE_BYTES;

/// What one settled read produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReadOutcome {
    Text {
        content: String,
        bytes: u64,
        mtime_ms: u64,
    },
    /// The file was still moving; the caller re-arms it (up to `MAX_RESETTLES`).
    Unsettled,
    Opaque {
        bytes: u64,
        reason: OpaqueReason,
    },
    /// The path is gone; on a change arm the matching unlink is on its way.
    Missing,
}

/// Size and mtime at one instant, used to prove the file held still across the read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileStamp {
    pub(crate) len: u64,
    pub(crate) mtime_ms: u64,
}

/// True when the file moved under the read: either stamp changed, or the bytes
/// read do not match the size that was advertised.
pub(crate) fn stamp_moved(before: &FileStamp, after: &FileStamp, read_len: usize) -> bool {
    before.len != after.len || before.mtime_ms != after.mtime_ms || read_len as u64 != after.len
}

/// Reads a file the settle queue considers quiet, proving it held still across
/// the read. Pure `std::fs`: the caller runs it inside `spawn_blocking` (ADR-009).
///
/// `expect_nonempty_baseline` is set when a baseline with content already
/// exists, which makes an empty read the first half of a truncate-then-write
/// rather than an honest emptying.
///
/// `accept_moving` is the final attempt after `MAX_RESETTLES`: a file that is
/// still moving is published as the text on disk right now - honest and
/// bounded - rather than held back forever.
pub(crate) fn read_settled(
    abs_path: &Path,
    expect_nonempty_baseline: bool,
    accept_moving: bool,
) -> ReadOutcome {
    let before = match stamp(abs_path) {
        Ok(Some(stamp)) => stamp,
        Ok(None) => return ReadOutcome::Missing,
        Err(bytes) => return unreadable(bytes),
    };
    if before.len > MAX_DELTA_FILE_BYTES {
        return ReadOutcome::Opaque {
            bytes: before.len,
            reason: OpaqueReason::TooLarge,
        };
    }

    let bytes = match read_bounded(abs_path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return ReadOutcome::Missing,
        Err(_) => return unreadable(before.len),
    };
    if bytes.len() as u64 > MAX_DELTA_FILE_BYTES {
        // The file grew past the bound between the stat and the read. Report the
        // size it has now rather than the one byte of overflow that proved it.
        let bytes = match stamp(abs_path) {
            Ok(Some(stamp)) => stamp.len,
            _ => bytes.len() as u64,
        };
        return ReadOutcome::Opaque {
            bytes,
            reason: OpaqueReason::TooLarge,
        };
    }

    let after = match stamp(abs_path) {
        Ok(Some(stamp)) => stamp,
        Ok(None) => return ReadOutcome::Missing,
        Err(_) => return unreadable(before.len),
    };
    if !accept_moving {
        if stamp_moved(&before, &after, bytes.len()) {
            return ReadOutcome::Unsettled;
        }
        if bytes.is_empty() && expect_nonempty_baseline {
            return ReadOutcome::Unsettled;
        }
    }

    // The length actually read; `stamp_moved` has already proved it equals
    // `after.len` on the settled path.
    let len = bytes.len() as u64;
    if bytes.contains(&0) {
        return binary(len);
    }
    match String::from_utf8(bytes) {
        Ok(content) => ReadOutcome::Text {
            content,
            bytes: len,
            mtime_ms: after.mtime_ms,
        },
        Err(_) => binary(len),
    }
}

/// Reads at most `MAX_DELTA_FILE_BYTES + 1` bytes: a file that grew past the
/// bound between the stat and the read costs one byte of overflow rather than
/// the whole file, and that one byte is what proves it is `Opaque(tooLarge)`.
fn read_bounded(abs_path: &Path) -> io::Result<Vec<u8>> {
    let file = fs::File::open(abs_path)?;
    let mut bytes = Vec::new();
    file.take(MAX_DELTA_FILE_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// `Ok(None)` means the path is gone; `Err(len)` means the stat itself failed.
fn stamp(abs_path: &Path) -> Result<Option<FileStamp>, u64> {
    let metadata = match fs::metadata(abs_path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(0),
    };
    if !metadata.is_file() {
        return Err(metadata.len());
    }
    let mtime_ms = metadata
        .modified()
        .ok()
        .and_then(|mtime| mtime.duration_since(UNIX_EPOCH).ok())
        .map(|since| u64::try_from(since.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0);
    Ok(Some(FileStamp {
        len: metadata.len(),
        mtime_ms,
    }))
}

fn unreadable(bytes: u64) -> ReadOutcome {
    ReadOutcome::Opaque {
        bytes,
        reason: OpaqueReason::Unreadable,
    }
}

fn binary(bytes: u64) -> ReadOutcome {
    ReadOutcome::Opaque {
        bytes,
        reason: OpaqueReason::Binary,
    }
}
