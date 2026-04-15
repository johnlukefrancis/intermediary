// Path: src-tauri/src/lib/agent/supervisor/wsl_runtime.rs
// Description: Shared WSL supervisor timing constants

use std::time::Duration;

pub(super) const WSL_TERMINATE_TERM_GRACE: Duration = Duration::from_millis(750);
pub(super) const WSL_TERMINATE_POLL: Duration = Duration::from_millis(50);
pub(super) const WSL_STALE_RETRY_BACKOFF: Duration = Duration::from_millis(300);
