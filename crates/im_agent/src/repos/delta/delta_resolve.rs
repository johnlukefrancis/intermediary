// Path: crates/im_agent/src/repos/delta/delta_resolve.rs
// Description: Resolves one settled change into a fileDelta payload - baseline ladder, settled read, image metadata

use std::io;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use im_bundle::cancel::BundleCancelToken;
use tokio::sync::Semaphore;

use crate::protocol::{DeltaBaseline, DeltaPayload, FileKind, OpaqueReason};
use crate::repos::mime_type_for_path;
use crate::source_control::read_index_blob;

use super::delta_stamp::{now_stamp, stamp_from_ms, stamp_of};
use super::{
    all_added_patch, all_removed_patch, compute_patch, read_settled, BaselineCache, PatchOutput,
    PendingChange, PendingOp, ReadOutcome, DIFF_DEADLINE, MAX_RESETTLES, READ_DEADLINE,
};

pub(super) struct ResolveContext<'a> {
    pub(super) root: &'a Path,
    pub(super) cache: &'a mut BaselineCache,
    pub(super) permits: &'a Semaphore,
    /// Cancelled when the worker stops, so a `git show` started for a baseline
    /// dies with the watcher instead of outliving it.
    pub(super) cancel: &'a BundleCancelToken,
    /// False when the burst budget refused this change: the resolver must not
    /// read the file, diff it, or spawn `git show`. Only a delete reaches here
    /// that way, and it publishes a bare `Gone` so the deletion still lands.
    pub(super) may_spawn: bool,
}

pub(super) enum Resolution {
    Emit {
        payload: DeltaPayload,
        mtime: String,
        /// A read or index failure worth one `warn`, even when a payload was still produced.
        failure: Option<String>,
    },
    /// The file was still moving; re-arm it.
    Resettle,
    /// Nothing to publish (the path vanished, or the worker is stopping).
    Drop,
}

fn emit(payload: DeltaPayload, mtime: String, failure: Option<String>) -> Resolution {
    Resolution::Emit {
        payload,
        mtime,
        failure,
    }
}

fn text_payload(patch: PatchOutput, baseline: DeltaBaseline) -> DeltaPayload {
    DeltaPayload::Text {
        patch: patch.patch,
        stats: patch.stats,
        baseline,
        truncated: patch.truncated,
    }
}

fn unreadable(bytes: u64, reason: String) -> Resolution {
    emit(
        DeltaPayload::Opaque {
            bytes,
            reason: OpaqueReason::Unreadable,
        },
        now_stamp(),
        Some(reason),
    )
}

pub(super) async fn resolve(
    context: &mut ResolveContext<'_>,
    change: &PendingChange,
) -> Resolution {
    match (change.kind, &change.op) {
        (FileKind::Image, PendingOp::Remove) => emit(DeltaPayload::Gone, now_stamp(), None),
        (FileKind::Image, _) => resolve_image(change).await,
        (_, PendingOp::Remove) => resolve_text_remove(context, &change.path).await,
        (_, _) => resolve_text_read(context, change).await,
    }
}

/// Metadata only: the UI fetches pixels through `readImageFile` under its own gate. An image
/// path always publishes an `Image` so it lands in a strip tile: a metadata failure other than
/// a vanished path is a zero-byte `Image` (its pixel fetch fails on its own) plus one warn.
pub(super) async fn resolve_image(change: &PendingChange) -> Resolution {
    let image = |bytes| DeltaPayload::Image {
        bytes,
        mime_type: mime_type_for_path(&change.path).map(str::to_string),
    };
    match tokio::fs::metadata(&change.abs_path).await {
        Ok(metadata) if metadata.is_dir() => Resolution::Drop,
        Ok(metadata) => {
            let mtime = metadata.modified().map_or_else(|_| now_stamp(), stamp_of);
            emit(image(metadata.len()), mtime, None)
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => Resolution::Drop,
        Err(err) => emit(image(0), now_stamp(), Some(err.to_string())),
    }
}

/// The deletion card: last served text, else the index blob, else `Gone`.
async fn resolve_text_remove(context: &mut ResolveContext<'_>, path: &str) -> Resolution {
    if !context.may_spawn {
        // The unbudgeted outcome: the deletion still prints, but as a bare
        // `Gone` card - no cached patch, no read, no `git show`.
        context.cache.remove(path);
        return emit(DeltaPayload::Gone, now_stamp(), None);
    }
    if let Some(old) = context.cache.remove(path) {
        let payload = text_payload(all_removed_patch(&old), DeltaBaseline::PreviousSighting);
        return emit(payload, now_stamp(), None);
    }
    match read_index_blob(context.root, path, Some(context.cancel.clone())).await {
        Ok(Some(text)) => emit(
            text_payload(all_removed_patch(&text), DeltaBaseline::Index),
            now_stamp(),
            None,
        ),
        Ok(None) => emit(DeltaPayload::Gone, now_stamp(), None),
        Err(err) => emit(
            DeltaPayload::Gone,
            now_stamp(),
            Some(err.message().to_string()),
        ),
    }
}

/// Settled read under the global permit, then the baseline ladder:
/// cache (renamed first) => index blob of the origin path => none.
async fn resolve_text_read(context: &mut ResolveContext<'_>, change: &PendingChange) -> Resolution {
    let path = change.path.as_str();
    let from = match &change.op {
        PendingOp::Rename { from } => Some(from.as_str()),
        _ => None,
    };
    move_rename_baseline(context.cache, from, path, change.resettles);
    let previous = context.cache.get(path);
    let expect_content = expect_nonempty(&change.op, previous.as_deref());

    let Ok(permit) = context.permits.acquire().await else {
        return Resolution::Drop;
    };
    let abs_path = change.abs_path.clone();
    let final_attempt = change.resettles >= MAX_RESETTLES;
    let read = tokio::time::timeout(
        READ_DEADLINE,
        tokio::task::spawn_blocking(move || read_settled(&abs_path, expect_content, final_attempt)),
    )
    .await;
    // The permit goes with the future either way: a read past `READ_DEADLINE`
    // must not hold one of the two process-wide slots hostage.
    drop(permit);
    let Ok(outcome) = read else {
        context.cache.remove(path);
        return unreadable(0, "read exceeded READ_DEADLINE".to_string());
    };

    let (content, mtime_ms) = match outcome {
        Ok(ReadOutcome::Text {
            content, mtime_ms, ..
        }) => (content, mtime_ms),
        Ok(ReadOutcome::Unsettled) => return Resolution::Resettle,
        Ok(ReadOutcome::Missing) => return Resolution::Drop,
        Ok(ReadOutcome::Opaque { bytes, reason }) => {
            context.cache.remove(path);
            let failure = (reason == OpaqueReason::Unreadable).then(|| "read failed".to_string());
            return emit(DeltaPayload::Opaque { bytes, reason }, now_stamp(), failure);
        }
        Err(err) => {
            context.cache.remove(path);
            return unreadable(0, format!("read task failed: {err}"));
        }
    };

    let new: Arc<str> = Arc::from(content);
    let (baseline, old, failure) = match previous {
        Some(old) => (DeltaBaseline::PreviousSighting, Some(old), None),
        None => match read_index_blob(
            context.root,
            from.unwrap_or(path),
            Some(context.cancel.clone()),
        )
        .await
        {
            Ok(Some(text)) => (DeltaBaseline::Index, Some(Arc::from(text)), None),
            Ok(None) => (DeltaBaseline::None, None, None),
            Err(err) => (DeltaBaseline::None, None, Some(err.message().to_string())),
        },
    };
    let patch = match old {
        Some(old) => {
            let new_text = Arc::clone(&new);
            let diffed = tokio::time::timeout(
                READ_DEADLINE,
                tokio::task::spawn_blocking(move || {
                    compute_patch(&old, &new_text, Instant::now() + DIFF_DEADLINE)
                }),
            )
            .await;
            match diffed {
                Ok(Ok(patch)) => patch,
                Ok(Err(err)) => {
                    context.cache.remove(path);
                    return unreadable(0, format!("diff task failed: {err}"));
                }
                Err(_) => {
                    context.cache.remove(path);
                    return unreadable(0, "diff exceeded READ_DEADLINE".to_string());
                }
            }
        }
        None => all_added_patch(&new),
    };
    context.cache.insert(path.to_string(), new);
    emit(
        text_payload(patch, baseline),
        stamp_from_ms(mtime_ms),
        failure,
    )
}

/// Moves the baseline with a renamed path, on the FIRST attempt only. A
/// re-settled rename finds nothing left at `from`, and `BaselineCache::rename`
/// treats a missing source as "no baseline any more" - so repeating the move
/// would evict the very baseline the first attempt carried across.
pub(super) fn move_rename_baseline(
    cache: &mut BaselineCache,
    from: Option<&str>,
    to: &str,
    resettles: u32,
) {
    if let (Some(from), 0) = (from, resettles) {
        cache.rename(from, to);
    }
}

/// Whether an empty read means "still being written" rather than "honestly
/// emptied". A modify says a file that already existed was rewritten, so an
/// empty read is the truncate half of a truncate-then-write even on a first
/// sighting where no baseline exists to compare against; an add or a rename
/// landing empty is taken at face value.
pub(super) fn expect_nonempty(op: &PendingOp, previous: Option<&str>) -> bool {
    matches!(op, PendingOp::Modify) || previous.is_some_and(|text| !text.is_empty())
}
