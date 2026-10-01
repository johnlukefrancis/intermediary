// Path: src-tauri/src/lib/terminal/session_spawn_cleanup.rs
// Description: Complete process-tree and PTY cleanup for terminal opens that fail after spawn

use super::frames::CloseReason;
use super::session::TerminalSession;
use super::session_spawn::SpawnedPty;
use super::transaction::TerminalTransaction;
use super::waiter_thread::terminate_and_observe;
use super::TerminalTree;
use portable_pty::MasterPty;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

pub const FAILED_OPEN_JOB_WAIT: Duration = Duration::from_millis(500);

pub fn terminate_failed_open(session: &TerminalSession) -> Option<std::io::Error> {
    #[cfg(test)]
    if super::spawn_faults::take("tree") {
        return Some(std::io::Error::other(
            "Injected process-tree cleanup failure",
        ));
    }
    session
        .job
        .terminate_and_observe(FAILED_OPEN_JOB_WAIT)
        .err()
}

pub fn close_unstarted_pty(session: &TerminalSession, reader: Option<Box<dyn Read + Send>>) {
    let _ = session.begin_close(CloseReason::OpenFailed);
    session.gate.release();
    #[cfg(unix)]
    let reader = {
        // A PTY reader keeps a master FD alive; draining it can wait on unresolved descendants forever.
        drop(reader);
        None
    };
    let (closer, retained_master) = match session.take_master().ok().flatten() {
        Some(master) => spawn_pty_drop(master, format!("terminal-open-pty-close-{}", session.id)),
        None => (None, None),
    };
    finish_pty_close(reader, closer, retained_master);
}

pub fn failed_open_detail(
    transaction: &TerminalTransaction,
    session: Arc<TerminalSession>,
    message: &str,
    cleanup: Option<std::io::Error>,
) -> String {
    if cleanup.is_some() {
        if let Err(err) = transaction.retain_failed_open(session) {
            return format!("{}; {err}", cleanup_detail(message, cleanup));
        }
    }
    cleanup_detail(message, cleanup)
}

pub fn discard_spawned(spawned: SpawnedPty, job: &TerminalTree) -> Option<std::io::Error> {
    let SpawnedPty {
        master,
        writer,
        reader,
        mut child,
    } = spawned;
    let cleanup = job.terminate_and_observe(FAILED_OPEN_JOB_WAIT).err();
    terminate_and_observe(&mut child);
    drop(writer);
    let (closer, retained_master) =
        spawn_pty_drop(master, "terminal-discard-pty-close".to_string());
    finish_pty_close(Some(reader), closer, retained_master);
    cleanup
}

pub fn cleanup_detail(message: &str, cleanup: Option<std::io::Error>) -> String {
    match cleanup {
        Some(error) => {
            format!("{message}; terminal process-tree cleanup could not be proved: {error}")
        }
        None => message.to_string(),
    }
}

/// Keeps ownership outside the closure until thread creation succeeds. On a
/// spawn failure the caller can drop its pipe ends before closing ConPTY.
fn spawn_pty_drop(
    master: Box<dyn MasterPty + Send>,
    name: String,
) -> (Option<JoinHandle<()>>, Option<Box<dyn MasterPty + Send>>) {
    let slot = Arc::new(Mutex::new(Some(master)));
    let worker_slot = slot.clone();
    match std::thread::Builder::new().name(name).spawn(move || {
        drop(
            worker_slot
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .take(),
        );
    }) {
        Ok(handle) => (Some(handle), None),
        Err(_) => (
            None,
            slot.lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .take(),
        ),
    }
}

fn finish_pty_close(
    mut reader: Option<Box<dyn Read + Send>>,
    closer: Option<JoinHandle<()>>,
    retained_master: Option<Box<dyn MasterPty + Send>>,
) {
    if let Some(master) = retained_master {
        // No closer thread exists. Break the output pipe before closing ConPTY
        // inline; otherwise draining waits for EOF from a still-open PTY.
        drop(reader);
        drop(master);
        return;
    }
    if let Some(reader) = reader.as_mut() {
        let _ = std::io::copy(reader, &mut std::io::sink());
    }
    drop(reader);
    if let Some(closer) = closer {
        let _ = closer.join();
    }
}
