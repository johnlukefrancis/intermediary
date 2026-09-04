// Path: src-tauri/src/lib/terminal/session.rs
// Description: One live terminal session: pty ends, child killer and Job Object, flow gate, exit record, phase and output channel

use super::exit_cell::ExitCell;
use super::flow_gate::FlowGate;
use super::frames::{CloseReason, TerminalExitFrame};
use super::output_sink::{OutputSink, PublishOutcome};
use im_bundle::process_job::JobHandle;
use portable_pty::{ChildKiller, MasterPty, PtySize};
use std::io::Write;
use std::sync::Mutex;
use tauri::ipc::{Channel, InvokeResponseBody};

pub const MIN_DIMENSION: u16 = 2;
pub const MAX_DIMENSION: u16 = 1000;

/// Running until the first close request; the reason of that request stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionPhase {
    Running,
    Closing(CloseReason),
}

/// Everything the spawn produced, handed over in one value.
pub struct SessionParts {
    pub id: String,
    pub pid: u32,
    pub job: JobHandle,
    pub master: Box<dyn MasterPty + Send>,
    pub writer: Box<dyn Write + Send>,
    pub killer: Box<dyn ChildKiller + Send + Sync>,
    pub channel: Channel<InvokeResponseBody>,
}

pub struct TerminalSession {
    pub id: String,
    pub pid: u32,
    /// Owner of the child's process tree; `terminate` is the escalation of a close
    pub job: JobHandle,
    pub gate: FlowGate,
    pub exit: ExitCell,
    pub sink: OutputSink,
    /// `None` once the pty was dropped (console-first close, or the waiter after exit)
    master: Mutex<Option<Box<dyn MasterPty + Send>>>,
    /// `None` once a close began: later writes are refused (I8)
    writer: Mutex<Option<Box<dyn Write + Send>>>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    phase: Mutex<SessionPhase>,
}

impl TerminalSession {
    pub fn new(parts: SessionParts) -> Self {
        Self {
            id: parts.id,
            pid: parts.pid,
            job: parts.job,
            gate: FlowGate::new(),
            exit: ExitCell::new(),
            sink: OutputSink::new(parts.channel),
            master: Mutex::new(Some(parts.master)),
            writer: Mutex::new(Some(parts.writer)),
            killer: Mutex::new(parts.killer),
            phase: Mutex::new(SessionPhase::Running),
        }
    }

    /// Writes keyboard input; the writer lock serialises concurrent writes. The
    /// phase is checked first so a closing session refuses input without
    /// touching the writer lock, which a wedged write may be holding.
    pub fn write(&self, bytes: &[u8]) -> Result<(), String> {
        if self.closing_reason()?.is_some() {
            return Err(self.closing_err());
        }
        let mut writer = self.writer.lock().map_err(|_| self.lock_err("writer"))?;
        let Some(writer) = writer.as_mut() else {
            return Err(self.closing_err());
        };
        writer
            .write_all(bytes)
            .and_then(|()| writer.flush())
            .map_err(|err| format!("Failed to write to terminal session {}: {err}", self.id))
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), String> {
        validate_size(cols, rows)?;
        let master = self.master.lock().map_err(|_| self.lock_err("pty"))?;
        let Some(master) = master.as_ref() else {
            return Err(self.closing_err());
        };
        master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|err| format!("Failed to resize terminal session {}: {err:#}", self.id))
    }

    pub fn ack(&self, consumed_total: u64) -> Result<(), String> {
        self.gate.ack(consumed_total)
    }

    /// Moves the session into `Closing(reason)`, after which writes are
    /// refused, and drops the input end when it is free. A write wedged in the
    /// pipe holds the writer lock; the close must not wait behind it, because
    /// only the pseudoconsole drop that follows can unstick that write. Returns
    /// `false` when a close was already under way; the earlier reason stands.
    pub fn begin_close(&self, reason: CloseReason) -> Result<bool, String> {
        let first = {
            let mut phase = self.phase.lock().map_err(|_| self.lock_err("phase"))?;
            let first = matches!(*phase, SessionPhase::Running);
            if first {
                *phase = SessionPhase::Closing(reason);
            }
            first
        };
        if let Ok(mut writer) = self.writer.try_lock() {
            drop(writer.take());
        }
        self.sink.detach();
        Ok(first)
    }

    pub fn closing_reason(&self) -> Result<Option<CloseReason>, String> {
        let phase = self.phase.lock().map_err(|_| self.lock_err("phase"))?;
        Ok(match *phase {
            SessionPhase::Running => None,
            SessionPhase::Closing(reason) => Some(reason),
        })
    }

    /// Takes the pty so the caller can drop it where blocking is acceptable.
    pub fn take_master(&self) -> Result<Option<Box<dyn MasterPty + Send>>, String> {
        Ok(self.master.lock().map_err(|_| self.lock_err("pty"))?.take())
    }

    /// Last resort of a close: ends the direct child alone. The kill's own
    /// result is not reported: portable-pty 0.9.0 inverts `TerminateProcess`'s
    /// result on Windows, so the exit record the caller waits on is the truth.
    pub fn kill_child(&self) -> Result<(), String> {
        let mut killer = self.killer.lock().map_err(|_| self.lock_err("killer"))?;
        let _ = killer.kill();
        Ok(())
    }

    /// The JSON frame that follows the last output byte on the same channel.
    pub fn send_exit_frame(
        &self,
        code: Option<u32>,
        reason: CloseReason,
    ) -> Result<PublishOutcome, String> {
        let frame = TerminalExitFrame::new(&self.id, code, reason);
        self.sink.publish_exit(&frame)
    }

    fn closing_err(&self) -> String {
        format!("Terminal session {} is closing", self.id)
    }

    fn lock_err(&self, what: &str) -> String {
        format!("Terminal session {} {what} lock poisoned", self.id)
    }
}

pub fn validate_size(cols: u16, rows: u16) -> Result<(), String> {
    let in_range = |value: u16| (MIN_DIMENSION..=MAX_DIMENSION).contains(&value);
    if in_range(cols) && in_range(rows) {
        Ok(())
    } else {
        Err(format!(
            "Terminal size {cols}x{rows} is outside {MIN_DIMENSION}..={MAX_DIMENSION}"
        ))
    }
}
