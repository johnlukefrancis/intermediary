// Path: src-tauri/src/lib/agent/supervisor/process_kill.rs
// Description: Blocking termination of a supervisor-owned process and the tree it started

use super::state::SupervisedChild;
use crate::obs::logging;
use im_bundle::process_job::JobHandle;
use std::process::Child;
use std::thread;
use std::time::{Duration, Instant};

pub(super) const KILL_WAIT_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const KILL_WAIT_POLL: Duration = Duration::from_millis(50);

pub(super) enum KillAndWaitOutcome {
    Exited(String),
    /// The child is still ours: it and its tree owner go back into the slot
    /// they came from, so the next stop can try again.
    Failed(SupervisedChild, String),
}

/// Ends the whole tree, then the child. The tree owner goes first because it is
/// the only thing that reaches the agent's descendants; `Child::kill` reaches
/// the agent alone, and anything it started would outlive the app that spawned
/// it. A process we adopted rather than started has no owner, and the kill of
/// the direct child is then the whole story — logged, never silent.
pub(super) fn kill_and_wait(process: SupervisedChild) -> KillAndWaitOutcome {
    let SupervisedChild { mut child, job } = process;
    terminate_tree(job.as_ref(), child.id());

    if let Err(err) = child.kill() {
        match child.try_wait() {
            Ok(Some(status)) => return KillAndWaitOutcome::Exited(status.to_string()),
            Ok(None) => {
                return failed(child, job, format!("kill signal failed: {err}"));
            }
            Err(wait_err) => {
                return failed(
                    child,
                    job,
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
                    return failed(
                        child,
                        job,
                        format!(
                            "process did not exit within {}ms after kill",
                            KILL_WAIT_TIMEOUT.as_millis()
                        ),
                    );
                }
                thread::sleep(KILL_WAIT_POLL);
            }
            Err(err) => return failed(child, job, err.to_string()),
        }
    }
}

/// Kills everything the recorded process started. Called for its own sake when
/// the child has already exited on its own: the owner outlives the process it
/// was created for, and dropping it releases the tree instead of ending it (the
/// job carries no kill-on-close limit, by decision).
pub(super) fn terminate_tree(job: Option<&JobHandle>, pid: u32) {
    let Some(job) = job else {
        logging::log(
            "info",
            "agent",
            "kill_tree",
            &format!("pid={pid} outcome=no_tree_owner detail=\"no tree owner (adopted agent)\""),
        );
        return;
    };
    match job.terminate() {
        Ok(()) => logging::log(
            "info",
            "agent",
            "kill_tree",
            &format!("pid={pid} outcome=terminated"),
        ),
        Err(err) => logging::log(
            "warn",
            "agent",
            "kill_tree",
            &format!("pid={pid} outcome=failed error={err}"),
        ),
    }
}

/// Ends a process the supervisor cannot record — the tree first, then the child
/// — so nothing is dropped while still running.
pub(super) fn discard_process(process: SupervisedChild) {
    let SupervisedChild { mut child, job } = process;
    terminate_tree(job.as_ref(), child.id());
    let _ = child.kill();
    let _ = child.wait();
}

fn failed(child: Child, job: Option<JobHandle>, message: String) -> KillAndWaitOutcome {
    KillAndWaitOutcome::Failed(SupervisedChild { child, job }, message)
}

#[cfg(test)]
mod tests {
    use super::{kill_and_wait, KillAndWaitOutcome};
    use crate::agent::supervisor::state::{spawn_test_sleeper, SupervisedChild};
    use im_bundle::process_job::JobHandle;

    /// An adopted agent — recorded by port and token alone — has no tree owner,
    /// and the kill of the direct child is still the whole stop.
    #[test]
    fn a_process_with_no_tree_owner_is_still_killed_and_reaped() {
        let process = SupervisedChild::from(spawn_test_sleeper());
        assert!(matches!(
            kill_and_wait(process),
            KillAndWaitOutcome::Exited(_)
        ));
    }

    /// The owner is spent before the child is killed. Off Windows it owns
    /// nothing, so this proves the call path, not the kill.
    #[test]
    fn a_process_with_a_tree_owner_is_killed_after_its_tree() {
        let job = JobHandle::create().expect("job");
        let process = SupervisedChild::owned(spawn_test_sleeper(), job);
        assert!(matches!(
            kill_and_wait(process),
            KillAndWaitOutcome::Exited(_)
        ));
    }
}
