// Path: crates/im_agent/src/repos/delta/delta_limits.rs
// Description: Process-wide delta bounds - the one read-permit semaphore every repo's worker shares

use std::sync::Arc;

use tokio::sync::Semaphore;

use super::DELTA_READ_CONCURRENCY;

/// Process-wide delta bounds: created once on the runtime and cloned into every
/// watcher, so `DELTA_READ_CONCURRENCY` holds across repos, not per repo.
#[derive(Clone)]
pub struct DeltaLimits {
    pub(crate) read_permits: Arc<Semaphore>,
}

impl DeltaLimits {
    pub fn new() -> Self {
        Self {
            read_permits: Arc::new(Semaphore::new(DELTA_READ_CONCURRENCY)),
        }
    }
}

impl Default for DeltaLimits {
    fn default() -> Self {
        Self::new()
    }
}
