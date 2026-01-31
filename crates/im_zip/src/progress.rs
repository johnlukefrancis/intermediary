// Path: crates/im_zip/src/progress.rs
// Description: Throttled JSON progress reporter for stdout

use std::io::{self, Write};
use std::time::{Duration, Instant};

use serde::Serialize;

/// Minimum interval between progress updates (100ms = 10 updates/sec max).
const THROTTLE_INTERVAL: Duration = Duration::from_millis(100);

/// Progress message types emitted to stdout.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProgressMessage {
    /// Incremental progress update.
    Progress {
        files_done: u64,
        files_total: u64,
        phase: &'static str,
    },
    /// Final completion message.
    Done { bytes_written: u64, file_count: u64 },
}

/// Throttled progress reporter that emits JSON lines to stdout.
pub struct ProgressReporter {
    files_total: u64,
    files_done: u64,
    last_emit: Option<Instant>,
}

impl ProgressReporter {
    /// Create a new progress reporter.
    pub fn new(files_total: u64) -> Self {
        Self {
            files_total,
            files_done: 0,
            last_emit: None,
        }
    }

    /// Record that a file has been processed. May emit progress if throttle allows.
    pub fn file_done(&mut self) {
        self.files_done += 1;

        let now = Instant::now();
        let should_emit = match self.last_emit {
            None => true,
            Some(last) => now.duration_since(last) >= THROTTLE_INTERVAL,
        };

        if should_emit {
            self.emit_progress();
            self.last_emit = Some(now);
        }
    }

    /// Emit a progress message immediately.
    fn emit_progress(&self) {
        let msg = ProgressMessage::Progress {
            files_done: self.files_done,
            files_total: self.files_total,
            phase: "zipping",
        };
        emit_json(&msg);
    }

    /// Emit the final done message. Always emits regardless of throttle.
    pub fn emit_done(&self, bytes_written: u64) {
        let msg = ProgressMessage::Done {
            bytes_written,
            file_count: self.files_done,
        };
        emit_json(&msg);
    }
}

/// Write a JSON message to stdout followed by newline.
fn emit_json<T: Serialize>(msg: &T) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    // Errors writing to stdout are ignored - process may be piped to closed reader
    let _ = serde_json::to_writer(&mut handle, msg);
    let _ = handle.write_all(b"\n");
    let _ = handle.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_message_serialization() {
        let progress = ProgressMessage::Progress {
            files_done: 5,
            files_total: 10,
            phase: "zipping",
        };
        let json = serde_json::to_string(&progress).unwrap();
        assert!(json.contains(r#""type":"progress""#));
        assert!(json.contains(r#""files_done":5"#));

        let done = ProgressMessage::Done {
            bytes_written: 1000,
            file_count: 10,
        };
        let json = serde_json::to_string(&done).unwrap();
        assert!(json.contains(r#""type":"done""#));
        assert!(json.contains(r#""bytes_written":1000"#));
    }

    #[test]
    fn reporter_increments_count() {
        let mut reporter = ProgressReporter::new(10);
        assert_eq!(reporter.files_done, 0);
        reporter.file_done();
        assert_eq!(reporter.files_done, 1);
        reporter.file_done();
        assert_eq!(reporter.files_done, 2);
    }
}
