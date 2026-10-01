// Path: src-tauri/src/lib/terminal/transaction_receipts.rs
// Description: Final and unresolved terminal receipts retaining process ownership through recovery

use super::{
    CloseOutcome, CloseReason, ReapBundle, TerminalReceipt, TerminalSession, TerminalTransaction,
    TransactionPhase,
};
use std::sync::Arc;

impl TerminalTransaction {
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

    pub fn retain_reap(&self, bundle: ReapBundle) -> Result<(), String> {
        let mut state = self.lock()?;
        state.receipt = Some(TerminalReceipt {
            reason: bundle.close_reason.unwrap_or(CloseReason::ChildExit),
            outcome: Some(CloseOutcome::StillAlive),
        });
        state.retained_reap = Some(bundle);
        self.settled.notify_all();
        Ok(())
    }

    pub fn take_retained_reap(&self) -> Result<Option<ReapBundle>, String> {
        Ok(self.lock()?.retained_reap.take())
    }

    /// Failed worker creation has no worker bundle; the unresolved process owner still reserves its slot.
    pub fn retain_failed_open(&self, session: Arc<TerminalSession>) -> Result<(), String> {
        let mut state = self.lock()?;
        if state.session.is_some() {
            return Err(format!(
                "Terminal transaction {} already owns a runtime",
                self.id
            ));
        }
        state.session = Some(session);
        state.phase = TransactionPhase::Reaping;
        state.receipt = Some(TerminalReceipt {
            reason: state.close_reason.unwrap_or(CloseReason::OpenFailed),
            outcome: Some(CloseOutcome::StillAlive),
        });
        self.settled.notify_all();
        Ok(())
    }

    /// Failed cleanup retains process and worker ownership for an app-exit retry.
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
}
