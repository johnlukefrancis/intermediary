// Path: src-tauri/src/lib/agent/supervisor/process_kill.rs
// Description: Blocking child-process termination helpers for supervisor-owned processes

use std::process::Child;
use std::thread;
use std::time::{Duration, Instant};

pub(super) const KILL_WAIT_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const KILL_WAIT_POLL: Duration = Duration::from_millis(50);

pub(super) enum KillAndWaitOutcome {
    Exited(String),
    Failed(Child, String),
}

pub(super) fn kill_and_wait(mut child: Child) -> KillAndWaitOutcome {
    if let Err(err) = child.kill() {
        match child.try_wait() {
            Ok(Some(status)) => return KillAndWaitOutcome::Exited(status.to_string()),
            Ok(None) => {
                return KillAndWaitOutcome::Failed(child, format!("kill signal failed: {err}"));
            }
            Err(wait_err) => {
                return KillAndWaitOutcome::Failed(
                    child,
                    format!("kill signal failed: {err}; poll failed: {wait_err}"),
                );
            }
        }
    }

    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return KillAndWaitOutcome::Exited(status.to_string()),
            Ok(None) => {
                if start.elapsed() >= KILL_WAIT_TIMEOUT {
                    return KillAndWaitOutcome::Failed(
                        child,
                        format!(
                            "process did not exit within {}ms after kill",
                            KILL_WAIT_TIMEOUT.as_millis()
                        ),
                    );
                }
                thread::sleep(KILL_WAIT_POLL);
            }
            Err(err) => return KillAndWaitOutcome::Failed(child, err.to_string()),
        }
    }
}
