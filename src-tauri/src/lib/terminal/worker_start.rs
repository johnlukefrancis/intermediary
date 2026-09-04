// Path: src-tauri/src/lib/terminal/worker_start.rs
// Description: Start barrier ensuring terminal worker handles are retained before either worker runs

use std::sync::{Condvar, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Waiting,
    Run,
    Abort,
}

#[derive(Debug)]
pub struct WorkerStart {
    state: Mutex<Option<State>>,
    changed: Condvar,
}

impl WorkerStart {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(Some(State::Waiting)),
            changed: Condvar::new(),
        }
    }

    pub fn release(&self) {
        self.set(State::Run);
    }

    pub fn abort(&self) {
        self.set(State::Abort);
    }

    pub fn wait(&self) -> bool {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        while matches!(*state, Some(State::Waiting)) {
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(|poison| poison.into_inner());
        }
        matches!(*state, Some(State::Run))
    }

    fn set(&self, next: State) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        *state = Some(next);
        self.changed.notify_all();
    }
}
