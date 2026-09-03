// Path: src-tauri/src/lib/agent/supervisor/wsl_runtime.rs
// Description: Shared WSL supervisor timing constants

use super::graceful_stop::HOST_STOP_WAIT_BOUND;
use crate::agent::wsl_agent_termination::WslTerminateBudget;
use std::time::Duration;

/// The envelope one WSL emergency stop is allowed.
///
/// `drain_wait` is the host's own graceful-stop bound, not a second number: the
/// WSL agent's shutdown drain is bounded by the same 450 s emergency bound the
/// host agent uses (`im_agent::server::SHUTDOWN_EMERGENCY_BOUND`), and killing
/// into that drain is what orphans a hook still holding `.git/index.lock`. Both
/// waits therefore share one owner, so they can never drift apart.
///
/// `kill_grace` is only what the kernel needs to deliver SIGKILL to a tree that
/// is no longer draining and for `ps` to stop seeing it.
pub(super) const WSL_TERMINATE_BUDGET: WslTerminateBudget = WslTerminateBudget {
    drain_wait: HOST_STOP_WAIT_BOUND,
    kill_grace: Duration::from_millis(750),
};

pub(super) const WSL_STALE_RETRY_BACKOFF: Duration = Duration::from_millis(300);
