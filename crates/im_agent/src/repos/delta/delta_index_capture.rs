// Path: crates/im_agent/src/repos/delta/delta_index_capture.rs
// Description: The index rung of the baseline ladder - stage-0 blob captured before the worktree read, fingerprinted off the runtime when large

use std::sync::Arc;

use crate::protocol::DeltaBaseline;

use super::delta_resolve::ResolveContext;
use super::delta_resolve_text::acquire_owned;
use super::{
    Baseline, PendingChange, MAX_DELTA_FILE_BYTES, MAX_FINGERPRINT_FILE_BYTES, READ_DEADLINE,
};

/// Reuses the capture a previous attempt left on the change, else fetches it before the
/// worktree read; a failure is not captured, so a re-settle retries and it still logs.
pub(super) async fn capture_index_baseline(
    context: &mut ResolveContext<'_>,
    change: &mut PendingChange,
    origin: &str,
) -> (DeltaBaseline, Option<Baseline>, Option<String>) {
    let captured = match &change.index_baseline {
        Some(captured) => captured.clone(),
        None => {
            let mut fetched = fetch_index(context, origin).await;
            if matches!(fetched, Ok(None)) && origin != change.path {
                // An atomic save renames an unindexed temp over the destination.
                fetched = fetch_index(context, &change.path).await;
            }
            match fetched {
                Ok(baseline) => {
                    change.index_baseline = Some(baseline.clone());
                    baseline
                }
                Err(message) => return (DeltaBaseline::None, None, Some(message)),
            }
        }
    };
    match captured {
        Some(baseline) => (DeltaBaseline::Index, Some(baseline), None),
        None => (DeltaBaseline::None, None, None),
    }
}

/// The stage-0 text of `rel`, up to `MAX_FINGERPRINT_FILE_BYTES`. Above
/// `MAX_DELTA_FILE_BYTES` only its fingerprints are kept, hashed under a read permit.
async fn fetch_index(
    context: &mut ResolveContext<'_>,
    rel: &str,
) -> Result<Option<Baseline>, String> {
    let text = context
        .reads
        .index_text(
            context.root,
            rel,
            MAX_FINGERPRINT_FILE_BYTES,
            context.cancel.clone(),
        )
        .await
        .map_err(|err| err.message().to_string())?;
    let Some(text) = text else {
        return Ok(None);
    };
    let text: Arc<str> = Arc::from(text);
    if text.len() as u64 <= MAX_DELTA_FILE_BYTES {
        return Ok(Some(Baseline::Text(text)));
    }
    let permit = acquire_owned(context.permits)
        .await
        .map_err(|_| "permit wait for index fingerprints failed".to_string())?;
    let retained = tokio::time::timeout(
        READ_DEADLINE,
        tokio::task::spawn_blocking(move || {
            let _held = permit;
            Baseline::retain(text)
        }),
    )
    .await;
    match retained {
        Ok(Ok(baseline)) => Ok(baseline),
        Ok(Err(err)) => Err(format!("index fingerprint task failed: {err}")),
        Err(_) => Err("index fingerprints exceeded READ_DEADLINE".to_string()),
    }
}
