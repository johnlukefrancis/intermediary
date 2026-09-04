// Path: src-tauri/src/lib/terminal/session_spawn.rs
// Description: Resource-symmetric terminal spawn into an already-admitted transaction

use super::reader_thread::spawn_reader;
use super::registry::TerminalRegistry;
use super::session::{SessionParts, TerminalSession};
use super::session_spawn_cleanup::{
    cleanup_detail, close_unstarted_pty, discard_spawned, FAILED_OPEN_JOB_WAIT,
};
use super::shell::TerminalCommand;
use super::transaction::{TerminalTransaction, WorkerHandles};
use super::waiter_thread::{spawn_waiter, terminate_and_observe, PtyChild};
use super::worker_start::WorkerStart;
use im_bundle::process_job::JobHandle;
use portable_pty::{MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::Arc;
use tauri::ipc::{Channel, InvokeResponseBody};

pub struct SpawnSpec {
    pub session_id: String,
    pub command: TerminalCommand,
    pub cols: u16,
    pub rows: u16,
    pub channel: Channel<InvokeResponseBody>,
}

#[derive(Debug)]
pub struct SpawnError {
    pub phase: &'static str,
    pub message: String,
}

impl SpawnError {
    pub fn new(phase: &'static str, message: impl Into<String>) -> Self {
        Self {
            phase,
            message: message.into(),
        }
    }
}

pub(super) struct SpawnedPty {
    pub master: Box<dyn MasterPty + Send>,
    pub writer: Box<dyn Write + Send>,
    pub reader: Box<dyn Read + Send>,
    pub child: PtyChild,
}

pub fn spawn_session(
    registry: &TerminalRegistry,
    transaction: &Arc<TerminalTransaction>,
    spec: SpawnSpec,
) -> Result<Arc<TerminalSession>, SpawnError> {
    let job = JobHandle::create().map_err(|err| {
        SpawnError::new(
            "job_create",
            format!("Failed to create the terminal's process tree owner: {err}"),
        )
    })?;
    let size = PtySize {
        rows: spec.rows,
        cols: spec.cols,
        pixel_width: 0,
        pixel_height: 0,
    };
    let spawned = spawn_platform(spec.command, size, &job)?;
    let Some(pid) = spawned.child.process_id() else {
        let cleanup = discard_spawned(spawned, &job);
        return Err(SpawnError::new(
            "spawn",
            cleanup_detail("The shell started without a process id", cleanup),
        ));
    };
    let killer = spawned.child.clone_killer();
    let session = Arc::new(TerminalSession::new(SessionParts {
        id: spec.session_id,
        pid,
        job,
        master: spawned.master,
        writer: spawned.writer,
        killer,
        channel: spec.channel,
    }));
    let start = Arc::new(WorkerStart::new());
    let waiter = match spawn_waiter(
        registry.clone(),
        transaction.clone(),
        session.clone(),
        spawned.child,
        start.clone(),
    ) {
        Ok(waiter) => waiter,
        Err(mut err) => {
            let cleanup = session
                .job
                .terminate_and_observe(FAILED_OPEN_JOB_WAIT)
                .err();
            if let Some(mut child) = err.child.take() {
                terminate_and_observe(&mut child);
            }
            close_unstarted_pty(&session, Some(spawned.reader));
            return Err(SpawnError::new(
                "waiter_thread",
                cleanup_detail(&err.message, cleanup),
            ));
        }
    };
    let reader = match spawn_reader(
        registry.clone(),
        transaction.clone(),
        session.clone(),
        spawned.reader,
        start.clone(),
    ) {
        Ok(reader) => reader,
        Err(err) => {
            start.abort();
            let cleanup = session
                .job
                .terminate_and_observe(FAILED_OPEN_JOB_WAIT)
                .err();
            let _ = session.kill_child();
            let _ = waiter.join();
            close_unstarted_pty(&session, err.reader);
            return Err(SpawnError::new(
                "reader_thread",
                cleanup_detail(&err.message, cleanup),
            ));
        }
    };
    let workers = WorkerHandles { reader, waiter };
    let pending_close = match transaction.install(session.clone(), workers) {
        Ok(reason) => reason,
        Err(failure) => {
            start.abort();
            let cleanup = session
                .job
                .terminate_and_observe(FAILED_OPEN_JOB_WAIT)
                .err();
            let _ = session.kill_child();
            let _ = failure.workers.waiter.join();
            let _ = failure.workers.reader.join();
            close_unstarted_pty(&session, None);
            return Err(SpawnError::new(
                "register",
                cleanup_detail(&failure.message, cleanup),
            ));
        }
    };
    if let Some(reason) = pending_close {
        let _ = session.begin_close(reason);
        session.gate.release();
    }
    start.release();
    if pending_close.is_some() {
        registry
            .start_pending_reaper(transaction)
            .map_err(|err| SpawnError::new("reaper_thread", err))?;
        return Err(SpawnError::new(
            "register",
            "The terminal open was cancelled by shutdown or navigation",
        ));
    }
    Ok(session)
}

#[cfg(not(windows))]
fn spawn_platform(
    command: TerminalCommand,
    size: PtySize,
    _job: &JobHandle,
) -> Result<SpawnedPty, SpawnError> {
    use portable_pty::{native_pty_system, PtyPair};
    let PtyPair { master, slave } = native_pty_system().openpty(size).map_err(|err| {
        SpawnError::new(
            "openpty",
            format!("Failed to open a pseudoconsole: {err:#}"),
        )
    })?;
    let writer = master.take_writer().map_err(|err| {
        SpawnError::new(
            "take_writer",
            format!("Failed to open terminal input: {err:#}"),
        )
    })?;
    let reader = master.try_clone_reader().map_err(|err| {
        SpawnError::new(
            "clone_reader",
            format!("Failed to open terminal output: {err:#}"),
        )
    })?;
    let child = slave
        .spawn_command(command.into_portable())
        .map_err(|err| SpawnError::new("spawn", format!("Failed to start the shell: {err:#}")))?;
    drop(slave);
    Ok(SpawnedPty {
        master,
        writer,
        reader,
        child,
    })
}

#[cfg(windows)]
fn spawn_platform(
    command: TerminalCommand,
    size: PtySize,
    job: &JobHandle,
) -> Result<SpawnedPty, SpawnError> {
    super::windows_pty::spawn(command, size, job)
}
