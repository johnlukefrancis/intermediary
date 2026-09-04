// Path: src-tauri/src/lib/terminal/flow_gate.rs
// Description: Cumulative sent/consumed flow watermarks bounding terminal output publication

use std::sync::{Condvar, Mutex, MutexGuard};

/// Unacked bytes at which the reader stops handing chunks to the channel.
pub const HIGH_WATER: u64 = 512 * 1024;
/// Unacked bytes at which a paused reader resumes; the gap keeps it from flapping.
pub const LOW_WATER: u64 = 128 * 1024;

#[derive(Debug, Default)]
struct GateState {
    sent_total: u64,
    consumed_total: u64,
    paused: bool,
    released: bool,
}

/// Bounds how far the output channel can run ahead of the frontend (I5).
/// Once explicitly released the gate never pauses again: teardown and channel
/// detach use that route so the reader can drain privately to EOF (I3), while
/// natural exit stays gated until its final bounded output is delivered.
#[derive(Debug, Default)]
pub struct FlowGate {
    state: Mutex<GateState>,
    credit: Condvar,
}

impl FlowGate {
    pub fn new() -> Self {
        Self::default()
    }

    /// Charges bytes before publication; crossing the high-water mark pauses
    /// the next read. Overflow is an explicit protocol failure.
    pub fn charge(&self, bytes: u64) -> Result<(), String> {
        let mut state = self.lock()?;
        state.sent_total = state
            .sent_total
            .checked_add(bytes)
            .ok_or_else(|| "Terminal output sent watermark overflowed".to_string())?;
        if unacked(&state) >= HIGH_WATER && !state.released {
            state.paused = true;
        }
        Ok(())
    }

    /// Blocks while the window is full; returns once acks bring it under the
    /// low-water mark or the gate is released.
    pub fn wait_for_credit(&self) -> Result<(), String> {
        let mut state = self.lock()?;
        while state.paused && !state.released {
            state = self.credit.wait(state).map_err(|_| poisoned())?;
        }
        Ok(())
    }

    /// Advances the cumulative consumed watermark. Duplicate and stale ACKs
    /// are idempotent; a watermark beyond what Rust published is impossible.
    pub fn ack(&self, consumed_total: u64) -> Result<(), String> {
        let mut state = self.lock()?;
        if consumed_total > state.sent_total {
            return Err(format!(
                "Terminal consumed watermark {consumed_total} exceeds sent watermark {}",
                state.sent_total
            ));
        }
        if consumed_total <= state.consumed_total {
            return Ok(());
        }
        state.consumed_total = consumed_total;
        if state.paused && unacked(&state) <= LOW_WATER {
            state.paused = false;
            self.credit.notify_all();
        }
        Ok(())
    }

    /// Opens the gate for good. A poisoned lock is recovered here on purpose:
    /// release is the step every close path depends on, and the state it
    /// guards is three plain flags with no invariant a panic could break.
    pub fn release(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        state.released = true;
        state.paused = false;
        self.credit.notify_all();
    }

    #[cfg(test)]
    pub fn is_paused(&self) -> Result<bool, String> {
        Ok(self.lock()?.paused)
    }

    #[cfg(test)]
    pub fn unacked(&self) -> Result<u64, String> {
        let state = self.lock()?;
        Ok(unacked(&state))
    }

    fn lock(&self) -> Result<MutexGuard<'_, GateState>, String> {
        self.state.lock().map_err(|_| poisoned())
    }
}

fn unacked(state: &GateState) -> u64 {
    state.sent_total - state.consumed_total
}

fn poisoned() -> String {
    "Terminal flow gate lock poisoned".to_string()
}

#[cfg(test)]
mod tests {
    use super::{FlowGate, HIGH_WATER, LOW_WATER};
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn the_window_pauses_at_high_water_and_resumes_only_under_low_water() {
        let gate = FlowGate::new();
        gate.charge(HIGH_WATER - 1).expect("charge");
        assert!(!gate.is_paused().expect("paused"));
        gate.charge(1).expect("charge");
        assert!(gate.is_paused().expect("paused"));

        // Down to one byte above the low-water mark: still paused (hysteresis).
        gate.ack(HIGH_WATER - LOW_WATER - 1).expect("ack");
        assert!(gate.is_paused().expect("paused"));
        gate.ack(HIGH_WATER - LOW_WATER).expect("ack");
        assert!(!gate.is_paused().expect("paused"));
        assert_eq!(gate.unacked().expect("unacked"), LOW_WATER);
    }

    #[test]
    fn a_parked_reader_wakes_on_ack() {
        let gate = Arc::new(FlowGate::new());
        gate.charge(HIGH_WATER).expect("charge");
        let (woke, wake_rx) = mpsc::channel();
        let reader_gate = gate.clone();
        thread::spawn(move || {
            reader_gate.wait_for_credit().expect("wait");
            let _ = woke.send(());
        });
        assert!(wake_rx.recv_timeout(Duration::from_millis(200)).is_err());
        gate.ack(HIGH_WATER).expect("ack");
        assert!(wake_rx.recv_timeout(Duration::from_secs(2)).is_ok());
    }

    #[test]
    fn release_opens_the_gate_for_good() {
        let gate = Arc::new(FlowGate::new());
        gate.charge(HIGH_WATER).expect("charge");
        let (woke, wake_rx) = mpsc::channel();
        let reader_gate = gate.clone();
        thread::spawn(move || {
            reader_gate.wait_for_credit().expect("wait");
            let _ = woke.send(());
        });
        gate.release();
        assert!(wake_rx.recv_timeout(Duration::from_secs(2)).is_ok());
        // Charging past the mark after release never pauses again.
        gate.charge(HIGH_WATER * 2).expect("charge");
        assert!(!gate.is_paused().expect("paused"));
        gate.wait_for_credit().expect("wait returns at once");
    }

    #[test]
    fn stale_duplicate_and_impossible_watermarks_are_distinct() {
        let gate = FlowGate::new();
        gate.charge(100).expect("charge");
        gate.ack(40).expect("advance");
        gate.ack(40).expect("duplicate");
        gate.ack(20).expect("stale");
        assert_eq!(gate.unacked().expect("unacked"), 60);
        let error = gate.ack(101).expect_err("beyond sent");
        assert!(error.contains("exceeds sent watermark"), "{error}");
        assert_eq!(gate.unacked().expect("unacked"), 60);
    }
}
