// Path: src-tauri/src/lib/terminal/exit_cell.rs
// Description: Set-once exit record of a session's child, with bounded waits for the threads that need it

use super::frames::CloseReason;
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// What the waiter observed when the child ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExitRecord {
    pub code: Option<u32>,
    pub reason: CloseReason,
}

/// The first observation of the child's end wins; later ones are refused.
#[derive(Debug, Default)]
pub struct ExitCell {
    record: Mutex<Option<ExitRecord>>,
    settled: Condvar,
}

impl ExitCell {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the exit; returns `false` when a record already stood.
    pub fn set_once(&self, record: ExitRecord) -> Result<bool, String> {
        let mut slot = self.lock()?;
        if slot.is_some() {
            return Ok(false);
        }
        *slot = Some(record);
        self.settled.notify_all();
        Ok(true)
    }

    pub fn get(&self) -> Result<Option<ExitRecord>, String> {
        Ok(*self.lock()?)
    }

    pub fn wait_timeout(&self, timeout: Duration) -> Result<Option<ExitRecord>, String> {
        self.wait_until(Instant::now() + timeout)
    }

    /// Blocks until the record exists or `deadline` passes; never spins.
    pub fn wait_until(&self, deadline: Instant) -> Result<Option<ExitRecord>, String> {
        let mut slot = self.lock()?;
        loop {
            if let Some(record) = *slot {
                return Ok(Some(record));
            }
            let now = Instant::now();
            if now >= deadline {
                return Ok(None);
            }
            let (guard, _) = self
                .settled
                .wait_timeout(slot, deadline - now)
                .map_err(|_| poisoned())?;
            slot = guard;
        }
    }

    fn lock(&self) -> Result<MutexGuard<'_, Option<ExitRecord>>, String> {
        self.record.lock().map_err(|_| poisoned())
    }
}

fn poisoned() -> String {
    "Terminal exit record lock poisoned".to_string()
}

#[cfg(test)]
mod tests {
    use super::{ExitCell, ExitRecord};
    use crate::terminal::frames::CloseReason;
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};

    fn record(code: u32, reason: CloseReason) -> ExitRecord {
        ExitRecord {
            code: Some(code),
            reason,
        }
    }

    #[test]
    fn the_first_record_stands() {
        let cell = ExitCell::new();
        assert!(cell
            .set_once(record(0, CloseReason::ChildExit))
            .expect("set"));
        assert!(!cell.set_once(record(1, CloseReason::Closed)).expect("set"));
        assert_eq!(
            cell.get().expect("get"),
            Some(record(0, CloseReason::ChildExit))
        );
    }

    #[test]
    fn a_wait_ends_at_the_deadline_without_a_record() {
        let cell = ExitCell::new();
        let started = Instant::now();
        let result = cell.wait_timeout(Duration::from_millis(50)).expect("wait");
        assert_eq!(result, None);
        assert!(started.elapsed() >= Duration::from_millis(50));
    }

    #[test]
    fn a_wait_wakes_when_the_record_lands() {
        let cell = Arc::new(ExitCell::new());
        let setter = cell.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(30));
            let _ = setter.set_once(record(7, CloseReason::AppExit));
        });
        let result = cell.wait_timeout(Duration::from_secs(2)).expect("wait");
        assert_eq!(result, Some(record(7, CloseReason::AppExit)));
    }
}
