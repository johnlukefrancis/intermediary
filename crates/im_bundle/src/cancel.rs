// Path: crates/im_bundle/src/cancel.rs
// Description: Cooperative cancellation token for bundle scan and zip operations

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::error::{BundleError, Result};

#[derive(Debug, Clone, Default)]
pub struct BundleCancelToken {
    cancelled: Arc<AtomicBool>,
}

impl BundleCancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

pub(crate) fn check_cancelled(cancel_token: Option<&BundleCancelToken>) -> Result<()> {
    if cancel_token.is_some_and(BundleCancelToken::is_cancelled) {
        return Err(BundleError::Cancelled);
    }
    Ok(())
}
