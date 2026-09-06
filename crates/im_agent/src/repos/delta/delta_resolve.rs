// Path: crates/im_agent/src/repos/delta/delta_resolve.rs
// Description: Resolves one settled change into a fileDelta payload - dispatch by kind and op, image metadata, the deletion card

use std::io;
use std::path::Path;
use std::sync::Arc;

use im_bundle::cancel::BundleCancelToken;
use tokio::sync::Semaphore;

use crate::protocol::{DeltaBaseline, DeltaPayload, FileKind, OpaqueReason};
use crate::repos::mime_type_for_path;

use super::delta_reads::ReadSources;
use super::delta_resolve_text::resolve_text_read;
use super::delta_stamp::{ms_of, now_stamp, stamp_of};
use super::{
    all_removed_patch, BaselineCache, PatchOutput, PendingChange, PendingOp, READ_DEADLINE,
};

pub(super) struct ResolveContext<'a> {
    pub(super) root: &'a Path,
    pub(super) cache: &'a mut BaselineCache,
    /// The process-wide read permits. A permit is acquired OWNED and moved into
    /// the blocking job, so a `READ_DEADLINE` expiry abandons the result
    /// without freeing a slot the pool thread is still using.
    pub(super) permits: &'a Arc<Semaphore>,
    pub(super) reads: &'a Arc<dyn ReadSources>,
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

pub(super) fn emit(payload: DeltaPayload, mtime: String, failure: Option<String>) -> Resolution {
    Resolution::Emit {
        payload,
        mtime,
        failure,
    }
}

pub(super) fn text_payload(patch: PatchOutput, baseline: DeltaBaseline) -> DeltaPayload {
    DeltaPayload::Text {
        patch: patch.patch,
        stats: patch.stats,
        baseline,
        truncated: patch.truncated,
    }
}

pub(super) fn unreadable(bytes: u64, reason: String) -> Resolution {
    emit(
        DeltaPayload::Opaque {
            bytes,
            reason: OpaqueReason::Unreadable,
        },
        now_stamp(),
        Some(reason),
    )
}

/// `change` is mutable so a first sighting can keep the index blob it captured
/// across a re-settle (`PendingChange::index_baseline`).
pub(super) async fn resolve(
    context: &mut ResolveContext<'_>,
    change: &mut PendingChange,
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
/// a vanished path - a stat past `READ_DEADLINE` included - is a zero-byte `Image` (its pixel
/// fetch fails on its own) plus one warn. `mtime_ms` comes from the same stat as `bytes`, so
/// the UI can bind the pixels it fetches to the revision this card reported.
pub(super) async fn resolve_image(change: &PendingChange) -> Resolution {
    let image = |bytes, mtime_ms| DeltaPayload::Image {
        bytes,
        mime_type: mime_type_for_path(&change.path).map(str::to_string),
        mtime_ms,
    };
    let stat = tokio::time::timeout(READ_DEADLINE, tokio::fs::metadata(&change.abs_path)).await;
    match stat {
        Err(_) => emit(
            image(0, 0),
            now_stamp(),
            Some("metadata exceeded READ_DEADLINE".to_string()),
        ),
        Ok(Ok(metadata)) if metadata.is_dir() => Resolution::Drop,
        Ok(Ok(metadata)) => {
            let modified = metadata.modified().ok();
            let mtime = modified.map_or_else(now_stamp, stamp_of);
            let mtime_ms = modified.map_or(0, ms_of);
            emit(image(metadata.len(), mtime_ms), mtime, None)
        }
        Ok(Err(err)) if err.kind() == io::ErrorKind::NotFound => Resolution::Drop,
        Ok(Err(err)) => emit(image(0, 0), now_stamp(), Some(err.to_string())),
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
    match context
        .reads
        .index_text(context.root, path, context.cancel.clone())
        .await
    {
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
