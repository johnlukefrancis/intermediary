// Path: src-tauri/src/lib/terminal/session_close.rs
// Description: The one close routine of a session: console-first pty drop, bounded wait, Job Object escalation, last-resort kill

use super::frames::{CloseOutcome, CloseReason};
use super::session::TerminalSession;
use crate::obs::logging;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const TAB_EXIT_WAIT: Duration = Duration::from_secs(3);
const TAB_AFTER_TERMINATE: Duration = Duration::from_millis(500);
/// One deadline shared by every session closed at app exit (I4).
pub const APP_EXIT_SHARED_WAIT: Duration = Duration::from_millis(1500);
const APP_EXIT_AFTER_TERMINATE: Duration = Duration::from_millis(300);
const LAST_RESORT_WAIT: Duration = Duration::from_millis(100);

/// How long a close may wait at each stage.
#[derive(Debug, Clone, Copy)]
pub struct CloseBudget {
    /// The console-first stage waits for the child until this instant
    pub exit_deadline: Instant,
    /// How long to wait after the Job Object terminate
    pub after_terminate: Duration,
}

pub struct CloseExecution {
    pub outcome: CloseOutcome,
    pub pty_close: Option<JoinHandle<()>>,
}

impl CloseBudget {
    pub fn tab() -> Self {
        Self {
            exit_deadline: Instant::now() + TAB_EXIT_WAIT,
            after_terminate: TAB_AFTER_TERMINATE,
        }
    }

    /// Sessions closed at exit share `shared_deadline`, so the total stays
    /// bounded whatever their count.
    pub fn app_exit(shared_deadline: Instant) -> Self {
        Self {
            exit_deadline: shared_deadline,
            after_terminate: APP_EXIT_AFTER_TERMINATE,
        }
    }
}

/// Ends a session the way Windows Terminal does (I2): drop the pty so every
/// attached client receives CTRL_CLOSE, wait, then terminate the Job Object,
/// then kill the direct child. GUI apps the shell launched detach from the
/// console and survive the first stage. Safe to run more than once.
pub fn close_session(
    session: &Arc<TerminalSession>,
    reason: CloseReason,
    budget: CloseBudget,
) -> CloseExecution {
    let started = Instant::now();
    if let Err(err) = session.begin_close(reason) {
        fault(session, "begin_close", &err);
    }
    // I3: the reader must never sit on the gate while the console shuts down;
    // conhost only finishes once its output pipe is drained.
    session.gate.release();
    let pty_close = drop_pty_detached(session);

    let outcome = match session.exit.wait_until(budget.exit_deadline) {
        Ok(Some(record)) => CloseOutcome::Exited { code: record.code },
        Ok(None) => escalate(session, reason, budget),
        Err(err) => {
            fault(session, "exit_wait", &err);
            escalate(session, reason, budget)
        }
    };
    logging::log(
        "info",
        "terminal",
        "session_close",
        &format!(
            "id={} reason={} outcome={} elapsed_ms={}",
            session.id,
            reason_label(reason),
            outcome_label(outcome),
            started.elapsed().as_millis()
        ),
    );
    CloseExecution { outcome, pty_close }
}

fn escalate(session: &TerminalSession, reason: CloseReason, budget: CloseBudget) -> CloseOutcome {
    let job_result = session.job.terminate_and_observe(budget.after_terminate);
    let detail = match &job_result {
        Ok(()) => "job=terminated_observed_empty".to_string(),
        Err(err) => format!("job=terminate_or_observe_failed error=\"{err}\""),
    };
    logging::log(
        "warn",
        "terminal",
        "close_escalated",
        &format!(
            "id={} pid={} reason={} {detail}",
            session.id,
            session.pid,
            reason_label(reason)
        ),
    );
    if job_result.is_ok() {
        let first = session.exit.wait_timeout(LAST_RESORT_WAIT).ok().flatten();
        if first.is_none() {
            // The Windows Job is already observed empty; this also keeps the
            // inert non-Windows lifecycle oracle honest by ending its direct
            // child rather than waiting for the test process to exit itself.
            if let Err(err) = session.kill_child() {
                fault(session, "kill_child", &err);
            }
        }
        let code = first
            .or_else(|| session.exit.wait_timeout(LAST_RESORT_WAIT).ok().flatten())
            .and_then(|record| record.code);
        return CloseOutcome::Escalated { code };
    }
    if let Err(err) = session.kill_child() {
        fault(session, "kill_child", &err);
    }
    let _ = session.exit.wait_timeout(LAST_RESORT_WAIT);
    // A direct-child receipt cannot prove that descendants are gone after a
    // Job API failure. Keep this outcome explicitly unresolved.
    CloseOutcome::StillAlive
}

/// `ClosePseudoConsole` may block until conhost drains, so the drop runs on a
/// short-lived thread of its own: never on the reader, never on a command.
fn drop_pty_detached(session: &Arc<TerminalSession>) -> Option<JoinHandle<()>> {
    let master = match session.take_master() {
        Ok(Some(master)) => master,
        Ok(None) => return None,
        Err(err) => {
            fault(session, "take_master", &err);
            return None;
        }
    };
    let spawned = thread::Builder::new()
        .name(format!("terminal-pty-drop-{}", session.id))
        .spawn(move || drop(master));
    match spawned {
        Ok(handle) => Some(handle),
        Err(err) => {
            // The failed spawn dropped its closure, and the pty with it, inline.
            fault(session, "pty_drop_thread", &err.to_string());
            None
        }
    }
}

/// An internal failure of a session's own machinery (a poisoned lock, a
/// thread that would not start); logged once at the stage it surfaced.
pub fn fault(session: &TerminalSession, stage: &str, err: &str) {
    logging::log(
        "warn",
        "terminal",
        "session_fault",
        &format!("id={} stage={stage} error=\"{err}\"", session.id),
    );
}

/// The serde name of the reason, for log lines.
pub fn reason_label(reason: CloseReason) -> &'static str {
    match reason {
        CloseReason::ChildExit => "childExit",
        CloseReason::Closed => "closed",
        CloseReason::WebviewNavigation => "webviewNavigation",
        CloseReason::AppExit => "appExit",
        CloseReason::ReaderError => "readerError",
        CloseReason::OpenFailed => "openFailed",
    }
}

pub fn outcome_label(outcome: CloseOutcome) -> &'static str {
    match outcome {
        CloseOutcome::Exited { .. } => "exited",
        CloseOutcome::Escalated { .. } => "escalated",
        CloseOutcome::StillAlive => "stillAlive",
    }
}

#[cfg(test)]
mod tests {
    use super::{outcome_label, reason_label};
    use crate::terminal::frames::{CloseOutcome, CloseReason};

    /// The log labels are the wire names; a drift here would make the log and
    /// the frontend disagree about the same ending.
    #[test]
    fn labels_match_the_wire_names() {
        let reasons = [
            CloseReason::ChildExit,
            CloseReason::Closed,
            CloseReason::WebviewNavigation,
            CloseReason::AppExit,
            CloseReason::ReaderError,
            CloseReason::OpenFailed,
        ];
        for reason in reasons {
            let wire = serde_json::to_string(&reason).expect("serialize");
            assert_eq!(wire, format!("\"{}\"", reason_label(reason)));
        }
        let outcomes = [
            CloseOutcome::Exited { code: None },
            CloseOutcome::Escalated { code: None },
            CloseOutcome::StillAlive,
        ];
        for outcome in outcomes {
            let wire = serde_json::to_value(outcome).expect("serialize");
            assert_eq!(wire["outcome"], outcome_label(outcome));
        }
    }
}
