// Path: src-tauri/src/lib/terminal/session_recovery_tests.rs
// Description: Failed-open ownership and slow-first-pass shutdown recovery regressions

use super::frames::CloseOutcome;
use super::registry::TerminalRegistry;
use super::session_spawn::{spawn_session, SpawnSpec};
use super::shell::TerminalCommand;
use super::spawn_faults;
use super::transaction::TerminalTransaction;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::ipc::Channel;

fn failed_open(registry: &TerminalRegistry, worker: &'static str) -> Arc<TerminalTransaction> {
    let transaction = registry.admit(worker, 0).expect("admit");
    let directory = tempfile::tempdir().expect("readiness directory");
    let ready = directory.path().join("child_pid");
    spawn_faults::arm(worker, ready.clone());
    let result = spawn_session(
        registry,
        &transaction,
        SpawnSpec {
            session_id: worker.to_string(),
            command: TerminalCommand {
                program: "sh".into(),
                args: vec![
                    "-c".into(),
                    format!(
                        "trap '' HUP; sleep 30 & printf '%s' \"$!\" > '{}'; wait",
                        ready.display()
                    )
                    .into(),
                ],
                cwd: std::env::current_dir().expect("cwd"),
                env: std::env::vars_os().collect(),
            },
            cols: 80,
            rows: 24,
            channel: Channel::new(|_| Ok(())),
        },
    );
    let error = result.err().expect("injected failed open");
    assert!(error.phase.starts_with(worker), "{error:?}");
    assert!(error
        .message
        .contains("process-tree cleanup could not be proved"));
    registry
        .fail_open(&transaction)
        .expect("return opening error");
    assert_eq!(registry.session_count().expect("capacity"), 1);
    assert!(registry.running(worker).is_err());
    assert!(transaction
        .unresolved_session()
        .expect("retained runtime")
        .is_some());
    let descendant: i32 = std::fs::read_to_string(ready)
        .expect("descendant pid")
        .parse()
        .expect("pid");
    // The direct child has been reaped; its HUP-resistant descendant still needs the retained owner.
    assert_eq!(unsafe { libc::kill(descendant, 0) }, 0);
    assert!(matches!(
        transaction.wait_receipt().expect("receipt").outcome,
        Some(CloseOutcome::StillAlive)
    ));
    transaction
}

#[test]
fn worker_creation_failure_retains_the_owner_until_retry_proves_finality() {
    for worker in ["waiter", "reader"] {
        let registry = TerminalRegistry::default();
        let transaction = failed_open(&registry, worker);
        assert!(registry.admit(worker, 0).is_err());
        registry.shutdown_all_blocking().expect("retry cleanup");
        assert_eq!(registry.session_count().expect("released capacity"), 0);
        assert!(matches!(
            transaction.wait_receipt().expect("final receipt").outcome,
            Some(CloseOutcome::Escalated { .. })
        ));
    }
}

#[test]
fn slow_first_pass_does_not_consume_the_shared_retry_observation_budget() {
    let registry = TerminalRegistry::default();
    let _retained = failed_open(&registry, "reader");
    let opening = registry.admit("slow-opening", 0).expect("slow admission");
    let delayed_registry = registry.clone();
    let delayed = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(400));
        delayed_registry
            .fail_open(&opening)
            .expect("settle slow first pass");
    });
    let started = Instant::now();
    registry
        .shutdown_all_blocking()
        .expect("recovered shutdown");
    delayed.join().expect("delayed first pass");
    assert!(started.elapsed() >= Duration::from_millis(400));
    let retry = spawn_faults::retry_budget().expect("retry was invoked");
    assert!(
        retry >= Duration::from_millis(250),
        "retry budget: {retry:?}"
    );
    assert!(
        retry <= Duration::from_millis(300),
        "shared bound: {retry:?}"
    );
    assert_eq!(registry.session_count().expect("all finalized"), 0);
}
