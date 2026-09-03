// Path: crates/im_bundle/src/git_capture/command_stop.rs
// Description: Forced stop of a running Git child: process-group signalling on unix, kill-and-reap everywhere

use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

use super::command::KillPolicy;

const POLL_INTERVAL: Duration = Duration::from_millis(10);
const GRACEFUL_TERMINATION_WAIT: Duration = Duration::from_secs(5);

/// Gives the child its own process group so a stop reaches every grandchild
/// (hook, ssh, credential helper, git-remote-https) that inherited the output
/// pipes; a surviving grandchild would otherwise hold the runner's readers
/// open long after Git itself is gone.
#[cfg(unix)]
pub(super) fn own_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(not(unix))]
pub(super) fn own_process_group(_command: &mut Command) {}

/// Stops the child and everything in its process group, then reaps the child.
/// `Graceful` first asks Git to terminate so its lockfile cleanup runs; after
/// the wait, or immediately for `Immediate`, the whole group is killed.
pub(super) fn stop_child(child: &mut Child, kill_policy: KillPolicy) {
    if kill_policy == KillPolicy::Graceful && terminate_group(child) {
        let deadline = Instant::now() + GRACEFUL_TERMINATION_WAIT;
        while Instant::now() < deadline {
            match child.try_wait() {
                Ok(Some(_)) => {
                    // Git exited on its own; whatever it left in the group
                    // must not keep the output pipes open.
                    kill_group(child);
                    return;
                }
                Ok(None) => thread::sleep(POLL_INTERVAL),
                Err(_) => break,
            }
        }
    }
    kill_group(child);
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(unix)]
fn terminate_group(child: &Child) -> bool {
    signal_group(child, libc::SIGTERM)
}

#[cfg(unix)]
fn kill_group(child: &Child) {
    signal_group(child, libc::SIGKILL);
}

#[cfg(unix)]
fn signal_group(child: &Child, signal: libc::c_int) -> bool {
    let Ok(pid) = libc::pid_t::try_from(child.id()) else {
        return false;
    };
    // SAFETY: the group id is the pid of a child we spawned with
    // `process_group(0)`; the number stays reserved while any member of the
    // group is alive, and the direct child is reaped only after this call.
    unsafe { libc::kill(-pid, signal) == 0 }
}

#[cfg(not(unix))]
fn terminate_group(_child: &Child) -> bool {
    // Windows has no cooperative termination signal for console children;
    // the caller's timeout is the only bound and TerminateProcess is final.
    false
}

#[cfg(not(unix))]
fn kill_group(_child: &Child) {}
