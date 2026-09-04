// Path: src-tauri/src/lib/terminal/registry.rs
// Description: Atomic admission and lifecycle registry retaining every terminal transaction through its joined receipt

use super::frames::{CloseOutcome, CloseReason};
use super::output_sink::PublishOutcome;
use super::reaper;
use super::session::TerminalSession;
use super::session_close::CloseBudget;
use super::transaction::{TerminalReceipt, TerminalTransaction};
use crate::obs::logging;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
pub const MAX_SESSIONS: usize = 12;
#[derive(Default)]
struct Sessions {
    transactions: HashMap<String, Arc<TerminalTransaction>>,
    page_generation: u32,
    closed_for_exit: bool,
}
#[derive(Clone, Default)]
pub struct TerminalRegistry {
    sessions: Arc<Mutex<Sessions>>,
}
impl TerminalRegistry {
    pub fn page_generation(&self) -> Result<u32, String> {
        Ok(self.lock()?.page_generation)
    }

    #[cfg(test)]
    pub fn session_count(&self) -> Result<usize, String> {
        Ok(self.lock()?.transactions.len())
    }

    pub fn admit(&self, id: &str, generation: u32) -> Result<Arc<TerminalTransaction>, String> {
        let mut sessions = self.lock()?;
        if sessions.closed_for_exit {
            return Err("The app is exiting; no terminal can open".to_string());
        }
        if sessions.page_generation != generation {
            return Err("The page navigated while the terminal was opening".to_string());
        }
        if sessions.transactions.len() >= MAX_SESSIONS {
            return Err(format!("Terminal session limit reached ({MAX_SESSIONS})"));
        }
        if sessions.transactions.contains_key(id) {
            return Err(format!("Terminal session {id} already exists"));
        }
        let transaction = Arc::new(TerminalTransaction::opening(id.to_string()));
        sessions
            .transactions
            .insert(id.to_string(), transaction.clone());
        Ok(transaction)
    }

    pub fn running(&self, id: &str) -> Result<Arc<TerminalSession>, String> {
        self.transaction(id)?.running_session()
    }

    pub fn acknowledge(&self, id: &str, consumed_total: u64) -> Result<(), String> {
        self.transaction(id)?.ack_session()?.ack(consumed_total)
    }

    pub fn close(&self, id: &str, reason: CloseReason) -> Result<CloseOutcome, String> {
        let transaction = self.transaction(id)?;
        if let Err(err) = self.request_close(&transaction, reason, CloseBudget::tab()) {
            logging::log("error", "terminal", "session_fault", &err);
            self.reap_inline(&transaction)?;
        }
        Ok(transaction
            .wait_receipt()?
            .outcome
            .unwrap_or(CloseOutcome::Exited { code: None }))
    }

    pub fn request_close_detached(
        &self,
        transaction: &Arc<TerminalTransaction>,
        reason: CloseReason,
    ) -> Result<(), String> {
        self.request_close(transaction, reason, CloseBudget::tab())
    }

    pub fn request_natural_reap(
        &self,
        transaction: &Arc<TerminalTransaction>,
    ) -> Result<(), String> {
        self.start_reaper(transaction, true)
    }

    pub fn fail_open(&self, transaction: &Arc<TerminalTransaction>) -> Result<(), String> {
        let mut sessions = self.lock()?;
        if transaction.complete_open_failure()?.is_some() {
            remove_exact(&mut sessions, transaction);
        }
        Ok(())
    }

    pub fn start_pending_reaper(
        &self,
        transaction: &Arc<TerminalTransaction>,
    ) -> Result<(), String> {
        match self.start_reaper(transaction, false) {
            Ok(()) => Ok(()),
            Err(err) => {
                logging::log("error", "terminal", "session_fault", &err);
                self.reap_inline(transaction)
            }
        }
    }

    pub fn finalize(
        &self,
        transaction: &Arc<TerminalTransaction>,
        session: &Arc<TerminalSession>,
        receipt: TerminalReceipt,
    ) -> Result<(), String> {
        match session.send_exit_frame(
            session.exit.get()?.and_then(|record| record.code),
            receipt.reason,
        ) {
            Ok(PublishOutcome::Published | PublishOutcome::Detached) => {}
            Err(err) => logging::log(
                "warn",
                "terminal",
                "channel_send_failed",
                &format!("id={} error=\"{err}\"", session.id),
            ),
        }
        let unresolved = matches!(receipt.outcome, Some(CloseOutcome::StillAlive));
        transaction.complete(receipt)?;
        if unresolved {
            return Ok(());
        }
        let mut sessions = self.lock()?;
        remove_exact(&mut sessions, transaction);
        Ok(())
    }

    pub(super) fn retry_unresolved(
        &self,
        transaction: &Arc<TerminalTransaction>,
        timeout: Duration,
    ) -> Result<(), String> {
        let Some(session) = transaction.unresolved_session()? else {
            return Ok(());
        };
        session.job.terminate_and_observe(timeout).map_err(|err| {
            format!(
                "Terminal session {} process tree remains unresolved: {err}",
                transaction.id
            )
        })?;
        let code = session.exit.get()?.and_then(|record| record.code);
        if transaction.resolve_still_alive(CloseOutcome::Escalated { code })? {
            let mut sessions = self.lock()?;
            remove_exact(&mut sessions, transaction);
        }
        Ok(())
    }

    pub(super) fn request_close(
        &self,
        transaction: &Arc<TerminalTransaction>,
        reason: CloseReason,
        budget: CloseBudget,
    ) -> Result<(), String> {
        if let Some(request) = transaction.request_close(reason, budget)? {
            if let Some(session) = request.session {
                session.begin_close(request.reason)?;
                session.gate.release();
            }
        }
        self.start_reaper(transaction, false)
    }

    fn start_reaper(
        &self,
        transaction: &Arc<TerminalTransaction>,
        natural: bool,
    ) -> Result<(), String> {
        if !transaction.claim_reaper(natural)? {
            return Ok(());
        }
        let registry = self.clone();
        let owned = transaction.clone();
        // The runtime blocking pool is the external joiner. Scheduling has no
        // fallible per-session OS-thread creation seam, so a reader or waiter
        // cannot be stranded holding its own JoinHandle after such a failure.
        // The transaction receipt, not this task handle, is authoritative.
        drop(tauri::async_runtime::spawn_blocking(move || {
            reaper::run(registry, owned)
        }));
        Ok(())
    }

    pub(super) fn reap_inline(&self, transaction: &Arc<TerminalTransaction>) -> Result<(), String> {
        if transaction.claim_reaper(false)? {
            reaper::run(self.clone(), transaction.clone());
        }
        Ok(())
    }

    fn transaction(&self, id: &str) -> Result<Arc<TerminalTransaction>, String> {
        self.lock()?
            .transactions
            .get(id)
            .cloned()
            .ok_or_else(|| format!("Unknown terminal session {id}"))
    }

    pub(super) fn navigation_snapshot(&self) -> Result<Vec<Arc<TerminalTransaction>>, String> {
        let mut sessions = self.lock()?;
        sessions.page_generation = sessions.page_generation.wrapping_add(1);
        Ok(sessions.transactions.values().cloned().collect())
    }

    pub(super) fn shutdown_snapshot(&self) -> Result<Vec<Arc<TerminalTransaction>>, String> {
        let mut sessions = self.lock()?;
        sessions.closed_for_exit = true;
        Ok(sessions.transactions.values().cloned().collect())
    }

    fn lock(&self) -> Result<MutexGuard<'_, Sessions>, String> {
        self.sessions
            .lock()
            .map_err(|_| "Terminal registry lock poisoned".to_string())
    }
}

fn remove_exact(sessions: &mut Sessions, transaction: &Arc<TerminalTransaction>) {
    if sessions
        .transactions
        .get(&transaction.id)
        .is_some_and(|current| Arc::ptr_eq(current, transaction))
    {
        sessions.transactions.remove(&transaction.id);
    }
}
