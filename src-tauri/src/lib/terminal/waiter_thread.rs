// Path: src-tauri/src/lib/terminal/waiter_thread.rs
// Description: Retained child waiter that records exit and requests the single external reaper

use super::exit_cell::{ExitCell, ExitRecord};
use super::frames::CloseReason;
use super::registry::TerminalRegistry;
use super::session::TerminalSession;
use super::session_close::fault;
use super::transaction::TerminalTransaction;
use super::worker_start::WorkerStart;
use portable_pty::Child;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

pub type PtyChild = Box<dyn Child + Send + Sync>;

pub struct WaiterSpawnError {
    pub message: String,
    pub child: Option<PtyChild>,
}

pub fn spawn_waiter(
    registry: TerminalRegistry,
    transaction: Arc<TerminalTransaction>,
    session: Arc<TerminalSession>,
    child: PtyChild,
    start: Arc<WorkerStart>,
) -> Result<JoinHandle<()>, WaiterSpawnError> {
    let child_slot = Arc::new(Mutex::new(Some(child)));
    let worker_slot = child_slot.clone();
    match thread::Builder::new()
        .name(format!("terminal-waiter-{}", session.id))
        .spawn(move || run(registry, transaction, session, worker_slot, start))
    {
        Ok(handle) => Ok(handle),
        Err(err) => Err(WaiterSpawnError {
            message: format!("Failed to start the terminal waiter thread: {err}"),
            child: child_slot
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .take(),
        }),
    }
}

fn run(
    registry: TerminalRegistry,
    transaction: Arc<TerminalTransaction>,
    session: Arc<TerminalSession>,
    child_slot: Arc<Mutex<Option<PtyChild>>>,
    start: Arc<WorkerStart>,
) {
    let Some(mut child) = child_slot
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .take()
    else {
        fault(
            &session,
            "child_owner",
            "waiter child handle was unavailable",
        );
        return;
    };
    if !start.wait() {
        terminate_and_observe(&mut child);
        return;
    }

    let code = match child.wait() {
        Ok(status) => Some(status.exit_code()),
        Err(err) => {
            fault(&session, "child_wait", &err.to_string());
            None
        }
    };
    let reason = match session.closing_reason() {
        Ok(Some(reason)) => reason,
        Ok(None) => CloseReason::ChildExit,
        Err(err) => {
            fault(&session, "closing_reason", &err);
            CloseReason::ChildExit
        }
    };
    set_exit(&session.exit, code, reason, &session);
    match session.take_master() {
        Ok(master) => drop(master),
        Err(err) => fault(&session, "take_master", &err),
    }
    if let Err(err) = registry.request_natural_reap(&transaction) {
        fault(&session, "natural_reap", &err);
    }
}

fn set_exit(exit: &ExitCell, code: Option<u32>, reason: CloseReason, session: &TerminalSession) {
    if let Err(err) = exit.set_once(ExitRecord { code, reason }) {
        fault(session, "exit_record", &err);
    }
}

pub fn terminate_and_observe(child: &mut PtyChild) {
    let _ = child.kill();
    let _ = child.wait();
}
