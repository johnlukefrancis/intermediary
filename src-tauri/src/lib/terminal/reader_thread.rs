// Path: src-tauri/src/lib/terminal/reader_thread.rs
// Description: Retained terminal reader that drains to EOF and reports its final bounded-output result

use super::frames::CloseReason;
use super::output_sink::PublishOutcome;
use super::registry::TerminalRegistry;
use super::session::TerminalSession;
use super::session_close::fault;
use super::transaction::TerminalTransaction;
use super::worker_start::WorkerStart;
use crate::obs::logging;
use std::io::{ErrorKind, Read};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const READ_CHUNK: usize = 16 * 1024;
const EXIT_RECORD_WAIT: Duration = Duration::from_secs(2);

#[derive(Debug)]
pub struct ReaderResult {
    pub bytes_out: u64,
    pub error: Option<String>,
}

pub struct ReaderSpawnError {
    pub message: String,
    pub reader: Option<Box<dyn Read + Send>>,
}

pub fn spawn_reader(
    registry: TerminalRegistry,
    transaction: Arc<TerminalTransaction>,
    session: Arc<TerminalSession>,
    reader: Box<dyn Read + Send>,
    start: Arc<WorkerStart>,
) -> Result<JoinHandle<ReaderResult>, ReaderSpawnError> {
    let reader_slot = Arc::new(Mutex::new(Some(reader)));
    let worker_slot = reader_slot.clone();
    match thread::Builder::new()
        .name(format!("terminal-reader-{}", session.id))
        .spawn(move || {
            if !start.wait() {
                return ReaderResult {
                    bytes_out: 0,
                    error: None,
                };
            }
            let Some(reader) = worker_slot
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .take()
            else {
                return ReaderResult {
                    bytes_out: 0,
                    error: Some("terminal reader handle was unavailable".to_string()),
                };
            };
            run(registry, transaction, session, reader)
        }) {
        Ok(handle) => Ok(handle),
        Err(err) => Err(ReaderSpawnError {
            message: format!("Failed to start the terminal reader thread: {err}"),
            reader: reader_slot
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
    mut reader: Box<dyn Read + Send>,
) -> ReaderResult {
    let mut buffer = vec![0u8; READ_CHUNK];
    let mut sink_attached = true;
    let mut bytes_out = 0u64;
    let error = loop {
        if sink_attached {
            if let Err(err) = session.gate.wait_for_credit() {
                fault(&session, "flow_gate", &err);
                session.gate.release();
            }
        }
        match reader.read(&mut buffer) {
            Ok(0) => break None,
            Ok(count) => {
                bytes_out = bytes_out.saturating_add(count as u64);
                if !sink_attached {
                    continue;
                }
                match session.sink.publish(&buffer[..count], &session.gate) {
                    Ok(PublishOutcome::Published) => {}
                    Ok(PublishOutcome::Detached) => {
                        sink_attached = false;
                        session.gate.release();
                    }
                    Err(err) => {
                        sink_attached = false;
                        session.gate.release();
                        logging::log(
                            "warn",
                            "terminal",
                            "channel_send_failed",
                            &format!("id={} error=\"{err}\"", session.id),
                        );
                    }
                }
            }
            Err(err) if err.kind() == ErrorKind::Interrupted => continue,
            Err(err) => break Some(err.to_string()),
        }
    };

    if let Some(err) = &error {
        logging::log(
            "warn",
            "terminal",
            "reader_error",
            &format!("id={} error=\"{err}\"", session.id),
        );
        if let Err(close_err) =
            registry.request_close_detached(&transaction, CloseReason::ReaderError)
        {
            fault(&session, "reader_close", &close_err);
        }
    } else {
        match session.exit.wait_timeout(EXIT_RECORD_WAIT) {
            Ok(Some(_)) => {
                if let Err(err) = registry.request_natural_reap(&transaction) {
                    fault(&session, "reader_natural_reap", &err);
                }
            }
            Ok(None) => {
                if let Err(err) =
                    registry.request_close_detached(&transaction, CloseReason::ReaderError)
                {
                    fault(&session, "reader_eof_close", &err);
                }
            }
            Err(err) => {
                fault(&session, "exit_record", &err);
                if let Err(close_err) =
                    registry.request_close_detached(&transaction, CloseReason::ReaderError)
                {
                    fault(&session, "reader_eof_close", &close_err);
                }
            }
        }
    }
    ReaderResult { bytes_out, error }
}
