// Path: src-tauri/src/lib/terminal/reaper.rs
// Description: Short external terminal reaper joining process, PTY-close, reader, and waiter ownership

use super::frames::{CloseOutcome, CloseReason};
use super::reader_thread::ReaderResult;
use super::registry::TerminalRegistry;
use super::session_close::{close_session, fault, reason_label};
use super::transaction::{TerminalReceipt, TerminalTransaction};
use crate::obs::logging;
use std::sync::Arc;

pub fn run(registry: TerminalRegistry, transaction: Arc<TerminalTransaction>) {
    let bundle = match transaction.take_reap_bundle() {
        Ok(bundle) => bundle,
        Err(err) => {
            logging::log("error", "terminal", "session_fault", &err);
            return;
        }
    };
    let close = match (bundle.close_reason, bundle.close_budget) {
        (Some(reason), Some(budget)) => Some(close_session(&bundle.session, reason, budget)),
        _ => None,
    };
    let (outcome, pty_close) = match close {
        Some(execution) => (Some(execution.outcome), execution.pty_close),
        None => (None, None),
    };

    let waiter_ok = bundle.workers.waiter.join().is_ok();
    if !waiter_ok {
        fault(&bundle.session, "waiter_join", "terminal waiter panicked");
    }
    let reader = match bundle.workers.reader.join() {
        Ok(result) => result,
        Err(_) => {
            fault(&bundle.session, "reader_join", "terminal reader panicked");
            ReaderResult {
                bytes_out: 0,
                error: Some("terminal reader panicked".to_string()),
            }
        }
    };
    if let Some(handle) = pty_close {
        if handle.join().is_err() {
            fault(
                &bundle.session,
                "pty_close_join",
                "terminal PTY closer panicked",
            );
        }
    }

    let record = bundle.session.exit.get().unwrap_or_else(|err| {
        fault(&bundle.session, "exit_record", &err);
        None
    });
    let reason = bundle
        .close_reason
        .or_else(|| record.map(|value| value.reason))
        .unwrap_or(CloseReason::ReaderError);
    let outcome = match outcome {
        Some(outcome) => Some(outcome),
        None => Some(CloseOutcome::Exited {
            code: record.and_then(|value| value.code),
        }),
    };
    let receipt = TerminalReceipt { reason, outcome };
    if let Err(err) = registry.finalize(&transaction, &bundle.session, receipt) {
        fault(&bundle.session, "registry_finalize", &err);
        return;
    }
    logging::log(
        "info",
        "terminal",
        "session_exit",
        &format!(
            "id={} pid={} code={} reason={} bytes_out={} reader_error={}",
            bundle.session.id,
            bundle.session.pid,
            record
                .and_then(|value| value.code)
                .map(|code| code.to_string())
                .unwrap_or_else(|| "none".to_string()),
            reason_label(reason),
            reader.bytes_out,
            reader.error.as_deref().unwrap_or("none")
        ),
    );
}
