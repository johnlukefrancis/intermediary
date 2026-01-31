// Path: crates/im_bundle/src/progress.rs
// Description: Throttled NDJSON progress emitter for bundle scanning and zipping

use std::io::{self, Write};
use std::time::{Duration, Instant};

use serde::Serialize;

const THROTTLE_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProgressMessage {
    Progress {
        phase: &'static str,
        files_done: u64,
        files_total: u64,
    },
    Done {
        bytes_written: u64,
        file_count: u64,
        scan_ms: u128,
        zip_ms: u128,
    },
}

pub struct ProgressEmitter {
    last_emit: Option<Instant>,
}

impl ProgressEmitter {
    pub fn new() -> Self {
        Self { last_emit: None }
    }

    pub fn emit_progress(&mut self, phase: &'static str, files_done: u64, files_total: u64) {
        let now = Instant::now();
        let should_emit = match self.last_emit {
            None => true,
            Some(last) => now.duration_since(last) >= THROTTLE_INTERVAL,
        };

        if should_emit {
            let msg = ProgressMessage::Progress {
                phase,
                files_done,
                files_total,
            };
            emit_json(&msg);
            self.last_emit = Some(now);
        }
    }

    pub fn emit_done(&mut self, bytes_written: u64, file_count: u64, scan_ms: u128, zip_ms: u128) {
        let msg = ProgressMessage::Done {
            bytes_written,
            file_count,
            scan_ms,
            zip_ms,
        };
        emit_json(&msg);
        self.last_emit = None;
    }
}

fn emit_json<T: Serialize>(msg: &T) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = serde_json::to_writer(&mut handle, msg);
    let _ = handle.write_all(b"\n");
    let _ = handle.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_message_serialization() {
        let msg = ProgressMessage::Progress {
            phase: "scanning",
            files_done: 2,
            files_total: 0,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"progress\""));
        assert!(json.contains("\"phase\":\"scanning\""));
    }
}
