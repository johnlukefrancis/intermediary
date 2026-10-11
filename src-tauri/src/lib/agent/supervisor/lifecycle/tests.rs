// Path: src-tauri/src/lib/agent/supervisor/lifecycle/tests.rs
// Description: The lifecycle lock makes ensure/restart/stop/exit single-flight

use super::AgentSupervisor;
use std::sync::{mpsc, Arc};
use std::time::Duration;

/// A healthcheck ensure or stop arriving mid-restart must wait for the restart, never interleave.
#[test]
fn stop_waits_for_the_current_lifecycle_owner() {
    let supervisor = Arc::new(AgentSupervisor::default());
    let held = supervisor.lifecycle.try_lock().expect("idle lifecycle");
    let (sender, receiver) = mpsc::channel();
    let waiter = Arc::clone(&supervisor);
    let thread = std::thread::spawn(move || {
        let _ = sender.send(tauri::async_runtime::block_on(waiter.stop()));
    });

    assert!(
        receiver.recv_timeout(Duration::from_millis(200)).is_err(),
        "stop ran while another lifecycle operation held the lock"
    );
    drop(held);
    receiver
        .recv_timeout(Duration::from_secs(10))
        .expect("stop completes once the owner releases")
        .expect("empty stop succeeds");
    thread.join().expect("stop thread");
}
