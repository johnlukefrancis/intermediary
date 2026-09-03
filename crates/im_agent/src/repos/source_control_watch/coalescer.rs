// Path: crates/im_agent/src/repos/source_control_watch/coalescer.rs
// Description: Rate-limit sourceControlChanged emission with a guaranteed trailing event

use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::protocol::{AgentEvent, SourceControlChangedEvent};
use crate::server::EventBus;

pub(crate) const COALESCE_WINDOW: Duration = Duration::from_millis(250);

/// Leading plus trailing coalescing: the first change after an idle window
/// emits immediately, every change inside the window folds into one trailing
/// emit due at `last_emit + window`. Never more than one emit per window, and
/// the tail is never dropped — the event bus drops on lag, so the trailing
/// emit is owned here rather than by any subscriber.
pub(crate) struct SourceControlCoalescer {
    window: Duration,
    dirty: bool,
    last_emit: Option<Instant>,
}

impl SourceControlCoalescer {
    pub(crate) fn new(window: Duration) -> Self {
        Self {
            window,
            dirty: false,
            last_emit: None,
        }
    }

    /// Records a change. Returns true when the caller must emit now.
    pub(crate) fn mark(&mut self, now: Instant) -> bool {
        let window_open = match self.last_emit {
            None => true,
            Some(last) => now.saturating_duration_since(last) >= self.window,
        };
        if window_open {
            self.dirty = false;
            self.last_emit = Some(now);
            return true;
        }
        self.dirty = true;
        false
    }

    /// When the trailing emit falls due, while one is owed. `mark` always sets
    /// `last_emit`, so a dirty coalescer always has a deadline.
    pub(crate) fn pending_deadline(&self) -> Option<Instant> {
        if !self.dirty {
            return None;
        }
        self.last_emit.map(|last| last + self.window)
    }

    /// Returns true when the trailing emit is owed.
    pub(crate) fn flush(&mut self, now: Instant) -> bool {
        if !self.dirty {
            return false;
        }
        self.dirty = false;
        self.last_emit = Some(now);
        true
    }
}

/// The coalescer bound to one repo's event bus: the only place a
/// `sourceControlChanged` event is broadcast.
pub(crate) struct SourceControlSignal {
    repo_id: String,
    event_bus: EventBus,
    state: Mutex<SourceControlCoalescer>,
}

impl SourceControlSignal {
    pub(crate) fn new(repo_id: String, event_bus: EventBus, window: Duration) -> Self {
        Self {
            repo_id,
            event_bus,
            state: Mutex::new(SourceControlCoalescer::new(window)),
        }
    }

    pub(crate) fn mark_dirty(&self) {
        if self.locked().mark(Instant::now()) {
            self.emit();
        }
    }

    pub(crate) fn pending_deadline(&self) -> Option<Instant> {
        self.locked().pending_deadline()
    }

    pub(crate) fn flush(&self) {
        if self.locked().flush(Instant::now()) {
            self.emit();
        }
    }

    fn locked(&self) -> std::sync::MutexGuard<'_, SourceControlCoalescer> {
        // No await and no panic happens under this guard; recover rather than
        // unwrap so a poisoned lock cannot take the watcher down (ADR-008).
        self.state.lock().unwrap_or_else(|err| err.into_inner())
    }

    fn emit(&self) {
        self.event_bus
            .broadcast_event(AgentEvent::SourceControlChanged(
                SourceControlChangedEvent::new(self.repo_id.clone()),
            ));
    }
}

#[cfg(test)]
mod tests {
    use super::{SourceControlCoalescer, COALESCE_WINDOW};
    use std::time::{Duration, Instant};

    #[test]
    fn burst_emits_once_immediately_and_once_trailing() {
        let mut coalescer = SourceControlCoalescer::new(COALESCE_WINDOW);
        let start = Instant::now();

        let mut immediate = 0;
        for step in 0..50u64 {
            if coalescer.mark(start + Duration::from_millis(step * 2)) {
                immediate += 1;
            }
        }

        assert_eq!(immediate, 1, "one leading emit for the whole burst");
        assert_eq!(coalescer.pending_deadline(), Some(start + COALESCE_WINDOW));
        assert!(
            coalescer.flush(start + COALESCE_WINDOW),
            "trailing emit is owed"
        );
        assert_eq!(coalescer.pending_deadline(), None);
        assert!(!coalescer.flush(start + COALESCE_WINDOW));
    }

    #[test]
    fn mark_after_window_emits_immediately() {
        let mut coalescer = SourceControlCoalescer::new(COALESCE_WINDOW);
        let start = Instant::now();

        assert!(coalescer.mark(start));
        assert!(!coalescer.mark(start + Duration::from_millis(10)));
        assert!(coalescer.mark(start + COALESCE_WINDOW));
        assert_eq!(
            coalescer.pending_deadline(),
            None,
            "the immediate emit consumed the pending tail"
        );
    }

    #[test]
    fn flush_without_marks_does_not_emit() {
        let mut coalescer = SourceControlCoalescer::new(COALESCE_WINDOW);
        assert!(!coalescer.flush(Instant::now()));
        assert_eq!(coalescer.pending_deadline(), None);
    }
}
