// Path: src-tauri/src/lib/terminal/spawn_faults.rs
// Description: Test-only one-shot worker and cleanup failures scoped to the spawning test thread

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::time::Duration;

thread_local! {
    static WORKER: Cell<Option<&'static str>> = const { Cell::new(None) };
    static TREE: Cell<bool> = const { Cell::new(false) };
    static RETRY: Cell<Option<Duration>> = const { Cell::new(None) };
    static READY: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

pub fn arm(worker: &'static str, ready: PathBuf) {
    WORKER.set(Some(worker));
    TREE.set(true);
    RETRY.set(None);
    READY.set(Some(ready));
}

pub fn take(stage: &str) -> bool {
    if stage == "tree" {
        return TREE.replace(false);
    }
    if WORKER.get() == Some(stage) {
        WORKER.set(None);
        let ready = READY.take().expect("failed-open readiness marker");
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while std::fs::read_to_string(&ready).map_or(true, |text| text.trim().is_empty()) {
            assert!(
                std::time::Instant::now() < deadline,
                "test child never armed"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        return true;
    }
    false
}

pub fn record_retry(timeout: Duration) {
    RETRY.set(Some(timeout));
}

pub fn retry_budget() -> Option<Duration> {
    RETRY.get()
}
