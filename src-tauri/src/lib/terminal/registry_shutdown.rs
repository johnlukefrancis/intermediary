// Path: src-tauri/src/lib/terminal/registry_shutdown.rs
// Description: Navigation and app-exit drains over atomically captured terminal transactions

use super::frames::{CloseOutcome, CloseReason};
use super::registry::TerminalRegistry;
use super::session_close::{reason_label, CloseBudget, APP_EXIT_SHARED_WAIT};
use crate::obs::logging;
use std::time::Instant;

const APP_EXIT_UNRESOLVED_RETRY: std::time::Duration = std::time::Duration::from_millis(300);

impl TerminalRegistry {
    pub fn close_all_detached(&self, reason: CloseReason) {
        let transactions = match self.navigation_snapshot() {
            Ok(transactions) => transactions,
            Err(err) => {
                logging::log("error", "terminal", "page_navigation_close", &err);
                return;
            }
        };
        if !transactions.is_empty() {
            logging::log(
                "info",
                "terminal",
                "page_navigation_close",
                &format!(
                    "count={} reason={}",
                    transactions.len(),
                    reason_label(reason)
                ),
            );
        }
        for transaction in transactions {
            if let Err(err) = self.request_close_detached(&transaction, reason) {
                logging::log("error", "terminal", "page_navigation_close", &err);
                if let Err(inline_err) = self.reap_inline(&transaction) {
                    logging::log("error", "terminal", "page_navigation_close", &inline_err);
                }
            }
        }
    }

    pub fn shutdown_all_blocking(&self) -> Result<(), String> {
        let started = Instant::now();
        let transactions = match self.shutdown_snapshot() {
            Ok(transactions) => transactions,
            Err(err) => {
                logging::log("error", "terminal", "shutdown_all", &err);
                return Err(err);
            }
        };
        let deadline = Instant::now() + APP_EXIT_SHARED_WAIT;
        for transaction in &transactions {
            if let Err(err) = self.request_close(
                transaction,
                CloseReason::AppExit,
                CloseBudget::app_exit(deadline),
            ) {
                logging::log("error", "terminal", "shutdown_all", &err);
                if let Err(inline_err) = self.reap_inline(transaction) {
                    logging::log("error", "terminal", "shutdown_all", &inline_err);
                }
            }
        }
        let retry_deadline = Instant::now() + APP_EXIT_UNRESOLVED_RETRY;
        let mut receipts = Vec::with_capacity(transactions.len());
        let mut receipt_errors = Vec::new();
        for transaction in &transactions {
            match transaction.wait_receipt() {
                Ok(receipt) => {
                    if matches!(receipt.outcome, Some(CloseOutcome::StillAlive)) {
                        let remaining = retry_deadline.saturating_duration_since(Instant::now());
                        if let Err(err) = self.retry_unresolved(transaction, remaining) {
                            logging::log("error", "terminal", "shutdown_all", &err);
                        }
                    }
                    match transaction.wait_receipt() {
                        Ok(receipt) => receipts.push(receipt),
                        Err(err) => receipt_errors
                            .push(format!("id={} receipt_error=\"{err}\"", transaction.id)),
                    }
                }
                Err(err) => {
                    let detail = format!("id={} receipt_error=\"{err}\"", transaction.id);
                    logging::log("error", "terminal", "shutdown_all", &detail);
                    receipt_errors.push(detail);
                }
            }
        }
        let count = |wanted: fn(&CloseOutcome) -> bool| {
            receipts
                .iter()
                .filter_map(|receipt| receipt.outcome.as_ref())
                .filter(|outcome| wanted(outcome))
                .count()
        };
        logging::log(
            "info",
            "terminal",
            "shutdown_all",
            &format!(
                "exited={} escalated={} still_alive={} receipt_errors={} elapsed_ms={}",
                count(|outcome| matches!(outcome, CloseOutcome::Exited { .. })),
                count(|outcome| matches!(outcome, CloseOutcome::Escalated { .. })),
                count(|outcome| matches!(outcome, CloseOutcome::StillAlive)),
                receipt_errors.len(),
                started.elapsed().as_millis()
            ),
        );
        let unresolved = count(|outcome| matches!(outcome, CloseOutcome::StillAlive));
        if receipt_errors.is_empty() && unresolved == 0 {
            Ok(())
        } else {
            Err(format!(
                "Terminal shutdown lacked finality (receipt_errors={}, still_alive={unresolved})",
                receipt_errors.len()
            ))
        }
    }
}
