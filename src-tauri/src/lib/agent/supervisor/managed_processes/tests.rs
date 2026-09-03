// Path: src-tauri/src/lib/agent/supervisor/managed_processes/tests.rs
// Description: State transitions for the supervisor's recorded process and the tree owner it carries

use super::super::state::{
    process_state_mut, spawn_test_exited_child, spawn_test_sleeper, ProcessKind, SupervisedChild,
};
use super::AgentSupervisor;
use im_bundle::process_job::JobHandle;

/// The pid and whether a tree owner is recorded, read straight out of the slot.
fn recorded(supervisor: &AgentSupervisor, kind: ProcessKind) -> Option<(u32, bool)> {
    let mut state = supervisor.state.lock().expect("state lock");
    let process = process_state_mut(&mut state, kind).process.as_ref()?;
    Some((process.child.id(), process.job.is_some()))
}

fn owned_sleeper() -> SupervisedChild {
    SupervisedChild::owned(spawn_test_sleeper(), JobHandle::create().expect("job"))
}

/// The whole point of pairing them: a stop that took the child and left the job
/// behind could never reach the descendants, so they move together or not at
/// all.
#[test]
fn the_tree_owner_travels_with_the_child_through_take_and_restore() {
    let supervisor = AgentSupervisor::default();
    let process = owned_sleeper();
    let pid = process.child.id();
    supervisor
        .store_child(ProcessKind::Host, process)
        .expect("store");

    let taken = supervisor
        .take_child(ProcessKind::Host)
        .expect("take")
        .expect("recorded");
    assert_eq!(taken.child.id(), pid);
    assert!(taken.job.is_some());
    assert_eq!(recorded(&supervisor, ProcessKind::Host), None);

    supervisor
        .restore_child(ProcessKind::Host, taken)
        .expect("restore");
    assert_eq!(recorded(&supervisor, ProcessKind::Host), Some((pid, true)));

    tauri::async_runtime::block_on(
        supervisor.reconcile_recorded_child(ProcessKind::Host, "test_cleanup"),
    )
    .expect("reconcile");
}

/// One recorded process at a time: the stale owner is spent by the
/// reconciliation `replace_child` runs first, never left in the slot beside the
/// new one and never dropped while its tree is still running.
#[test]
fn replacing_a_running_child_records_only_the_new_one() {
    let supervisor = AgentSupervisor::default();
    let stale = owned_sleeper();
    let stale_pid = stale.child.id();
    supervisor
        .store_child(ProcessKind::Host, stale)
        .expect("store");

    let fresh = owned_sleeper();
    let fresh_pid = fresh.child.id();
    tauri::async_runtime::block_on(supervisor.replace_child(ProcessKind::Host, fresh))
        .expect("replace");

    assert_ne!(stale_pid, fresh_pid);
    assert_eq!(
        recorded(&supervisor, ProcessKind::Host),
        Some((fresh_pid, true))
    );

    tauri::async_runtime::block_on(
        supervisor.reconcile_recorded_child(ProcessKind::Host, "test_cleanup"),
    )
    .expect("reconcile");
}

/// A child that exited on its own still leaves an owner holding whatever it
/// started; reconciliation spends it and clears the slot either way.
#[test]
fn reconciling_an_exited_child_clears_the_slot() {
    let supervisor = AgentSupervisor::default();
    let process = SupervisedChild::owned(
        spawn_test_exited_child(),
        JobHandle::create().expect("job"),
    );
    supervisor
        .store_child(ProcessKind::Host, process)
        .expect("store");

    tauri::async_runtime::block_on(
        supervisor.reconcile_recorded_child(ProcessKind::Host, "test_exited"),
    )
    .expect("reconcile");
    assert_eq!(recorded(&supervisor, ProcessKind::Host), None);
}

/// A process the supervisor refuses to record is ended, not dropped: dropping
/// it would leave a live agent and a live tree that nothing can reach again.
#[test]
fn a_process_that_cannot_be_stored_is_ended_rather_than_dropped() {
    let supervisor = AgentSupervisor::default();
    let kept = owned_sleeper();
    let kept_pid = kept.child.id();
    supervisor
        .store_child(ProcessKind::Host, kept)
        .expect("store");

    let refused = owned_sleeper();
    let refused_pid = refused.child.id();
    let error = supervisor
        .store_child(ProcessKind::Host, refused)
        .expect_err("the occupied slot must refuse");
    assert!(error.contains("slot already occupied"), "{error}");
    assert_ne!(kept_pid, refused_pid);
    assert_eq!(
        recorded(&supervisor, ProcessKind::Host),
        Some((kept_pid, true))
    );

    tauri::async_runtime::block_on(
        supervisor.reconcile_recorded_child(ProcessKind::Host, "test_cleanup"),
    )
    .expect("reconcile");
}

/// The WSL launcher is recorded through the same slot with no tree owner: a
/// Windows job object around `wsl.exe` reaches nothing inside the distro.
#[test]
fn a_child_recorded_without_a_tree_owner_is_still_reconciled() {
    let supervisor = AgentSupervisor::default();
    let pid = {
        let process = SupervisedChild::from(spawn_test_sleeper());
        let pid = process.child.id();
        tauri::async_runtime::block_on(supervisor.replace_child(ProcessKind::Wsl, process.child))
            .expect("replace");
        pid
    };
    assert_eq!(recorded(&supervisor, ProcessKind::Wsl), Some((pid, false)));

    tauri::async_runtime::block_on(
        supervisor.reconcile_recorded_child(ProcessKind::Wsl, "test_cleanup"),
    )
    .expect("reconcile");
    assert_eq!(recorded(&supervisor, ProcessKind::Wsl), None);
}
