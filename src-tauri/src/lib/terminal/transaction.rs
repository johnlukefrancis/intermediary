// Path: src-tauri/src/lib/terminal/transaction.rs
// Description: One admitted terminal transaction from Opening through joined Terminal receipt

use super::frames::{CloseOutcome, CloseReason};
use super::reader_thread::ReaderResult;
use super::session::TerminalSession;
use super::session_close::CloseBudget;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionPhase {
    Opening,
    Running,
    Closing,
    Reaping,
    Terminal,
}

#[derive(Debug, Clone, Copy)]
pub struct TerminalReceipt {
    pub reason: CloseReason,
    pub outcome: Option<CloseOutcome>,
}

pub struct WorkerHandles {
    pub reader: JoinHandle<ReaderResult>,
    pub waiter: JoinHandle<()>,
}

pub struct InstallFailure {
    pub message: String,
    pub workers: WorkerHandles,
}

pub struct CloseRequest {
    pub session: Option<Arc<TerminalSession>>,
    pub reason: CloseReason,
}

pub struct ReapBundle {
    pub session: Arc<TerminalSession>,
    pub workers: WorkerHandles,
    pub close_reason: Option<CloseReason>,
    pub close_budget: Option<CloseBudget>,
}

struct TransactionState {
    phase: TransactionPhase,
    session: Option<Arc<TerminalSession>>,
    workers: Option<WorkerHandles>,
    close_reason: Option<CloseReason>,
    close_budget: Option<CloseBudget>,
    reaper_started: bool,
    receipt: Option<TerminalReceipt>,
}

pub struct TerminalTransaction {
    pub id: String,
    state: Mutex<TransactionState>,
    settled: Condvar,
}

impl TerminalTransaction {
    pub fn opening(id: String) -> Self {
        Self {
            id,
            state: Mutex::new(TransactionState {
                phase: TransactionPhase::Opening,
                session: None,
                workers: None,
                close_reason: None,
                close_budget: None,
                reaper_started: false,
                receipt: None,
            }),
            settled: Condvar::new(),
        }
    }

    /// Installs every long-lived resource before workers are released. Returns
    /// true when a close raced the open and a reaper must start immediately.
    pub fn install(
        &self,
        session: Arc<TerminalSession>,
        workers: WorkerHandles,
    ) -> Result<Option<CloseReason>, InstallFailure> {
        let mut state = match self.lock() {
            Ok(state) => state,
            Err(message) => return Err(InstallFailure { message, workers }),
        };
        if state.session.is_some() || state.receipt.is_some() {
            return Err(InstallFailure {
                message: format!("Terminal transaction {} cannot install twice", self.id),
                workers,
            });
        }
        state.session = Some(session);
        state.workers = Some(workers);
        let close_reason = state.close_reason;
        state.phase = if close_reason.is_some() {
            TransactionPhase::Closing
        } else {
            TransactionPhase::Running
        };
        Ok(close_reason)
    }

    /// The first ending reason stands. The caller detaches the returned
    /// session's output before releasing its flow gate.
    pub fn request_close(
        &self,
        reason: CloseReason,
        budget: CloseBudget,
    ) -> Result<Option<CloseRequest>, String> {
        let mut state = self.lock()?;
        if state.receipt.is_some() {
            return Ok(None);
        }
        if state.close_reason.is_none() {
            state.close_reason = Some(reason);
            state.close_budget = Some(budget);
        }
        if state.phase != TransactionPhase::Reaping {
            state.phase = TransactionPhase::Closing;
        }
        Ok(Some(CloseRequest {
            session: state.session.clone(),
            reason: state.close_reason.unwrap_or(reason),
        }))
    }

    pub fn running_session(&self) -> Result<Arc<TerminalSession>, String> {
        let state = self.lock()?;
        if state.phase != TransactionPhase::Running {
            return Err(format!("Terminal session {} is not running", self.id));
        }
        state
            .session
            .clone()
            .ok_or_else(|| format!("Terminal session {} has no runtime", self.id))
    }

    pub fn ack_session(&self) -> Result<Arc<TerminalSession>, String> {
        self.lock()?
            .session
            .clone()
            .ok_or_else(|| format!("Terminal session {} is still opening", self.id))
    }

    /// Claims the single external reaper after both workers are installed.
    pub fn claim_reaper(&self, natural: bool) -> Result<bool, String> {
        let mut state = self.lock()?;
        if state.reaper_started || state.receipt.is_some() {
            return Ok(false);
        }
        if state.session.is_none() || state.workers.is_none() {
            return Ok(false);
        }
        if natural && state.close_reason.is_none() {
            state.close_reason = Some(CloseReason::ChildExit);
            state.phase = TransactionPhase::Reaping;
        }
        state.reaper_started = true;
        Ok(true)
    }

    pub fn take_reap_bundle(&self) -> Result<ReapBundle, String> {
        let mut state = self.lock()?;
        state.phase = TransactionPhase::Reaping;
        let session = state
            .session
            .clone()
            .ok_or_else(|| format!("Terminal transaction {} has no runtime", self.id))?;
        let workers = state
            .workers
            .take()
            .ok_or_else(|| format!("Terminal transaction {} has no worker handles", self.id))?;
        Ok(ReapBundle {
            session,
            workers,
            close_reason: state.close_reason,
            close_budget: state.close_budget,
        })
    }

    pub fn complete(&self, receipt: TerminalReceipt) -> Result<(), String> {
        let mut state = self.lock()?;
        state.phase = if matches!(receipt.outcome, Some(CloseOutcome::StillAlive)) {
            TransactionPhase::Reaping
        } else {
            TransactionPhase::Terminal
        };
        state.receipt = Some(receipt);
        self.settled.notify_all();
        Ok(())
    }

    /// A failed Job escalation keeps the session owner resident. App exit may
    /// retry that one remaining process-tree receipt without recreating worker
    /// or PTY ownership that has already joined.
    pub fn unresolved_session(&self) -> Result<Option<Arc<TerminalSession>>, String> {
        let state = self.lock()?;
        Ok(matches!(
            state.receipt.and_then(|receipt| receipt.outcome),
            Some(CloseOutcome::StillAlive)
        )
        .then(|| state.session.clone())
        .flatten())
    }

    pub fn resolve_still_alive(&self, outcome: CloseOutcome) -> Result<bool, String> {
        let mut state = self.lock()?;
        let Some(mut receipt) = state.receipt else {
            return Ok(false);
        };
        if !matches!(receipt.outcome, Some(CloseOutcome::StillAlive)) {
            return Ok(false);
        }
        receipt.outcome = Some(outcome);
        state.receipt = Some(receipt);
        state.phase = TransactionPhase::Terminal;
        self.settled.notify_all();
        Ok(true)
    }

    /// Settles an admitted open only while no runtime has been installed.
    /// A concurrent close reason wins over the generic open-failure label.
    pub fn complete_open_failure(&self) -> Result<Option<TerminalReceipt>, String> {
        let mut state = self.lock()?;
        if state.session.is_some() || state.receipt.is_some() {
            return Ok(None);
        }
        let receipt = TerminalReceipt {
            reason: state.close_reason.unwrap_or(CloseReason::OpenFailed),
            outcome: None,
        };
        state.phase = TransactionPhase::Terminal;
        state.receipt = Some(receipt);
        self.settled.notify_all();
        Ok(Some(receipt))
    }

    pub fn wait_receipt(&self) -> Result<TerminalReceipt, String> {
        let mut state = self.lock()?;
        loop {
            if let Some(receipt) = state.receipt {
                return Ok(receipt);
            }
            state = self
                .settled
                .wait(state)
                .unwrap_or_else(|poison| poison.into_inner());
        }
    }

    #[cfg(test)]
    pub fn phase(&self) -> Result<TransactionPhase, String> {
        Ok(self.lock()?.phase)
    }

    fn lock(&self) -> Result<MutexGuard<'_, TransactionState>, String> {
        Ok(self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner()))
    }
}
