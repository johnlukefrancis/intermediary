// Path: src-tauri/src/lib/terminal/registry_tests.rs
// Description: Atomic admission and Opening-transaction regression tests for TerminalRegistry

use super::frames::{CloseOutcome, CloseReason};
use super::registry::{TerminalRegistry, MAX_SESSIONS};
use super::transaction::{TerminalReceipt, TransactionPhase};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

#[test]
fn opening_transactions_reserve_capacity_and_identity() {
    let registry = TerminalRegistry::default();
    for index in 0..MAX_SESSIONS {
        registry
            .admit(&format!("session-{index}"), 0)
            .expect("admit");
    }
    assert!(registry.admit("overflow", 0).is_err());
    assert_eq!(registry.session_count().expect("count"), MAX_SESSIONS);
}

#[test]
fn concurrent_admission_never_exceeds_the_bound() {
    let registry = TerminalRegistry::default();
    let start = Arc::new(Barrier::new(MAX_SESSIONS * 2));
    let workers = (0..MAX_SESSIONS * 2)
        .map(|index| {
            let registry = registry.clone();
            let start = start.clone();
            thread::spawn(move || {
                start.wait();
                registry.admit(&format!("racing-{index}"), 0).ok()
            })
        })
        .collect::<Vec<_>>();
    let admitted = workers
        .into_iter()
        .filter_map(|worker| worker.join().expect("admission worker"))
        .collect::<Vec<_>>();

    assert_eq!(admitted.len(), MAX_SESSIONS);
    assert_eq!(registry.session_count().expect("count"), MAX_SESSIONS);
    for transaction in admitted {
        registry.fail_open(&transaction).expect("settle opening");
    }
}

#[test]
fn navigation_and_open_failure_preserve_one_transaction_route() {
    let registry = TerminalRegistry::default();
    let transaction = registry.admit("opening", 0).expect("admit");
    registry.close_all_detached(CloseReason::WebviewNavigation);
    assert_eq!(
        transaction.phase().expect("phase"),
        TransactionPhase::Closing
    );
    registry.fail_open(&transaction).expect("fail open");
    assert_eq!(registry.session_count().expect("count"), 0);
    assert!(registry.admit("old-page", 0).is_err());
    assert_eq!(
        transaction.wait_receipt().expect("receipt").reason,
        CloseReason::WebviewNavigation
    );
}

#[test]
fn app_exit_waits_for_a_transaction_that_is_still_opening() {
    let registry = TerminalRegistry::default();
    let transaction = registry.admit("opening-at-exit", 0).expect("admit");
    let shutdown_registry = registry.clone();
    let shutdown = thread::spawn(move || shutdown_registry.shutdown_all_blocking());
    let deadline = Instant::now() + Duration::from_secs(2);
    while transaction.phase().expect("phase") != TransactionPhase::Closing {
        assert!(
            Instant::now() < deadline,
            "shutdown never claimed opening transaction"
        );
        thread::sleep(Duration::from_millis(5));
    }

    registry
        .fail_open(&transaction)
        .expect("settle failed open");
    shutdown
        .join()
        .expect("shutdown thread")
        .expect("shutdown receipt");
    assert_eq!(registry.session_count().expect("count"), 0);
    assert!(registry.admit("after-exit", 0).is_err());
    assert_eq!(
        transaction.wait_receipt().expect("receipt").reason,
        CloseReason::AppExit
    );
}

#[test]
fn unresolved_process_tree_receipt_blocks_supervisor_finality() {
    let registry = TerminalRegistry::default();
    let transaction = registry.admit("unresolved-job", 0).expect("admit");
    transaction
        .complete(TerminalReceipt {
            reason: CloseReason::AppExit,
            outcome: Some(CloseOutcome::StillAlive),
        })
        .expect("record unresolved receipt");

    let error = registry
        .shutdown_all_blocking()
        .expect_err("unresolved tree must block finality");
    assert!(error.contains("still_alive=1"), "{error}");
    assert_eq!(registry.session_count().expect("count"), 1);
    assert_eq!(
        transaction.phase().expect("phase"),
        TransactionPhase::Reaping
    );
}
