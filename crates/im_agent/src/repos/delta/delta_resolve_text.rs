// Path: crates/im_agent/src/repos/delta/delta_resolve_text.rs
// Description: The text arm of the resolver - index baseline captured before the worktree read, deadline-bounded owned read permits, the zero-length first-sighting rule, the baseline ladder and diff

use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::protocol::{DeltaBaseline, DeltaPayload, OpaqueReason};

use super::delta_resolve::{emit, text_payload, unreadable, Resolution, ResolveContext};
use super::delta_stamp::{now_stamp, stamp_from_ms};
use super::{
    all_added_patch, compute_patch, BaselineCache, PendingChange, PendingOp, ReadOutcome,
    DIFF_DEADLINE, MAX_RESETTLES, READ_DEADLINE,
};

/// The baseline ladder, in causal order: cache (renamed first) => index blob
/// of the origin path => none. For an uncached path the index is read BEFORE
/// the worktree and captured on the change, so the pair a `VS INDEX` card
/// compares was taken in one order and a re-settle diffs against the same
/// index text rather than an index a commit may have moved since.
pub(super) async fn resolve_text_read(
    context: &mut ResolveContext<'_>,
    change: &mut PendingChange,
) -> Resolution {
    let path = change.path.clone();
    let from = match &change.op {
        PendingOp::Rename { from } => Some(from.clone()),
        _ => None,
    };
    move_rename_baseline(context.cache, from.as_deref(), &path, change.resettles);
    let previous = context.cache.get(&path);
    let expect_content = expect_nonempty(&change.op, previous.as_deref());

    let (baseline, old, failure) = match previous {
        Some(old) => (DeltaBaseline::PreviousSighting, Some(old), None),
        None => capture_index_baseline(context, change, from.as_deref().unwrap_or(&path)).await,
    };

    // A stopped watcher cannot cancel an in-flight blocking read: the pool
    // thread finishes on its own and drops the permit when it does. What stops
    // is only our observing it (`READ_DEADLINE`) - an accepted boundary.
    let permit = match acquire_owned(context.permits).await {
        Ok(permit) => permit,
        Err(denied) => return permit_denied(context, &path, denied),
    };
    let reads = Arc::clone(context.reads);
    let abs_path = change.abs_path.clone();
    let final_attempt = change.resettles >= MAX_RESETTLES;
    let read = tokio::time::timeout(
        READ_DEADLINE,
        tokio::task::spawn_blocking(move || {
            let _held = permit;
            reads.settled_read(&abs_path, expect_content, final_attempt)
        }),
    )
    .await;
    let Ok(outcome) = read else {
        context.cache.remove(&path);
        return unreadable(0, "read exceeded READ_DEADLINE".to_string());
    };

    let (content, mtime_ms) = match outcome {
        Ok(ReadOutcome::Text {
            content, mtime_ms, ..
        }) => (content, mtime_ms),
        Ok(ReadOutcome::Unsettled) => return Resolution::Resettle,
        Ok(ReadOutcome::Missing) => return Resolution::Drop,
        Ok(ReadOutcome::Opaque { bytes, reason }) => {
            context.cache.remove(&path);
            let failure = (reason == OpaqueReason::Unreadable).then(|| "read failed".to_string());
            return emit(DeltaPayload::Opaque { bytes, reason }, now_stamp(), failure);
        }
        Err(err) => {
            context.cache.remove(&path);
            return unreadable(0, format!("read task failed: {err}"));
        }
    };

    let new: Arc<str> = Arc::from(content);
    if settled_empty_against_index(baseline, old.as_deref(), &new) {
        // The zero-length rule: no card and no baseline, so the next sighting
        // is still a first sighting and diffs against the index again.
        return Resolution::Drop;
    }
    let patch = match old {
        Some(old) => {
            let permit = match acquire_owned(context.permits).await {
                Ok(permit) => permit,
                Err(denied) => return permit_denied(context, &path, denied),
            };
            let new_text = Arc::clone(&new);
            let diffed = tokio::time::timeout(
                READ_DEADLINE,
                tokio::task::spawn_blocking(move || {
                    let _held = permit;
                    compute_patch(&old, &new_text, Instant::now() + DIFF_DEADLINE)
                }),
            )
            .await;
            match diffed {
                Ok(Ok(patch)) => patch,
                Ok(Err(err)) => {
                    context.cache.remove(&path);
                    return unreadable(0, format!("diff task failed: {err}"));
                }
                Err(_) => {
                    context.cache.remove(&path);
                    return unreadable(0, "diff exceeded READ_DEADLINE".to_string());
                }
            }
        }
        None => all_added_patch(&new),
    };
    context.cache.insert(path, new);
    emit(
        text_payload(patch, baseline),
        stamp_from_ms(mtime_ms),
        failure,
    )
}

/// The index half of a first sighting. Reuses the capture a previous attempt
/// left on the change; otherwise fetches it now, before the worktree is read,
/// and stores it. A Git failure is not captured, so a re-settle tries again
/// and the failure still reaches the log through this attempt's `failure`.
async fn capture_index_baseline(
    context: &mut ResolveContext<'_>,
    change: &mut PendingChange,
    origin: &str,
) -> (DeltaBaseline, Option<Arc<str>>, Option<String>) {
    let captured = match &change.index_baseline {
        Some(captured) => captured.clone(),
        None => match context
            .reads
            .index_text(context.root, origin, context.cancel.clone())
            .await
        {
            Ok(text) => {
                let text = text.map(Arc::from);
                change.index_baseline = Some(text.clone());
                text
            }
            Err(err) => {
                return (DeltaBaseline::None, None, Some(err.message().to_string()));
            }
        },
    };
    match captured {
        Some(text) => (DeltaBaseline::Index, Some(text), None),
        None => (DeltaBaseline::None, None, None),
    }
}

/// Why a read permit was not obtained.
enum PermitDenied {
    /// The semaphore was closed, which never happens in the product: the
    /// permits live on the runtime for the life of the process.
    Closed,
    /// Both permits stayed held for `READ_DEADLINE`: a stalled filesystem is
    /// holding the blocking pool, and this repo must not park behind it.
    Expired,
}

/// Waits at most `READ_DEADLINE` for one of the process-wide read permits.
async fn acquire_owned(permits: &Arc<Semaphore>) -> Result<OwnedSemaphorePermit, PermitDenied> {
    match tokio::time::timeout(READ_DEADLINE, Arc::clone(permits).acquire_owned()).await {
        Ok(Ok(permit)) => Ok(permit),
        Ok(Err(_closed)) => Err(PermitDenied::Closed),
        Err(_elapsed) => Err(PermitDenied::Expired),
    }
}

/// A closed semaphore is the worker stopping: nothing to publish. An expired
/// wait degrades to an opaque card and evicts the baseline, like every other
/// `READ_DEADLINE` expiry, so a stalled filesystem costs cards, not workers.
fn permit_denied(context: &mut ResolveContext<'_>, path: &str, denied: PermitDenied) -> Resolution {
    match denied {
        PermitDenied::Closed => Resolution::Drop,
        PermitDenied::Expired => {
            context.cache.remove(path);
            unreadable(0, "permit wait exceeded READ_DEADLINE".to_string())
        }
    }
}

/// The zero-length rule. A first sighting (no cached baseline, so the index
/// blob is the baseline) whose settled text is empty while that index text is
/// not gets no card and stores no baseline: an empty file against a non-empty
/// index is the truncate half of a write the settle window could not outlast,
/// and an all-removed card would be false. Once a baseline is cached
/// (`PreviousSighting`) an empty read is an honest emptying and prints.
pub(super) fn settled_empty_against_index(
    baseline: DeltaBaseline,
    old: Option<&str>,
    new: &str,
) -> bool {
    baseline == DeltaBaseline::Index && new.is_empty() && old.is_some_and(|text| !text.is_empty())
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
