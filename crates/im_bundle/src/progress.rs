// Path: crates/im_bundle/src/progress.rs
// Description: Throttled NDJSON progress emitter for bundle scanning and zipping

use std::time::{Duration, Instant};

use serde::Serialize;

use crate::progress_sink::{ProgressSink, StdoutProgressSink};

const THROTTLE_INTERVAL: Duration = Duration::from_millis(100);
const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(250);

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProgressMessage {
    Progress {
        phase: &'static str,
        files_done: u64,
        files_total: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        current_file: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        current_bytes_done: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        current_bytes_total: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        bytes_done_total_best_effort: Option<u64>,
    },
    Done {
        bytes_written: u64,
        file_count: u64,
        scan_ms: u128,
        zip_ms: u128,
    },
}

pub struct ProgressEmitter {
    sink: Box<dyn ProgressSink>,
    last_emit: Option<Instant>,
    last_phase: Option<&'static str>,
    last_heartbeat: Option<Instant>,
    last_heartbeat_file: Option<String>,
}

impl ProgressEmitter {
    pub fn new() -> Self {
        Self {
            sink: Box::new(StdoutProgressSink::new()),
            last_emit: None,
            last_phase: None,
            last_heartbeat: None,
            last_heartbeat_file: None,
        }
    }

    pub fn with_sink(sink: Box<dyn ProgressSink>) -> Self {
        Self {
            sink,
            last_emit: None,
            last_phase: None,
            last_heartbeat: None,
            last_heartbeat_file: None,
        }
    }

    pub fn emit_progress(&mut self, phase: &'static str, files_done: u64, files_total: u64) {
        self.emit_progress_internal(
            phase,
            files_done,
            files_total,
            None,
            None,
            None,
            None,
            ProgressThrottle::Standard,
            false,
        );
    }

    pub fn emit_file_progress(
        &mut self,
        phase: &'static str,
        files_done: u64,
        files_total: u64,
        current_file: &str,
        current_bytes_done: u64,
        current_bytes_total: Option<u64>,
        bytes_done_total_best_effort: u64,
        force: bool,
    ) {
        self.emit_progress_internal(
            phase,
            files_done,
            files_total,
            Some(current_file),
            Some(current_bytes_done),
            current_bytes_total,
            Some(bytes_done_total_best_effort),
            ProgressThrottle::Heartbeat,
            force,
        );
    }

    fn emit_progress_internal(
        &mut self,
        phase: &'static str,
        files_done: u64,
        files_total: u64,
        current_file: Option<&str>,
        current_bytes_done: Option<u64>,
        current_bytes_total: Option<u64>,
        bytes_done_total_best_effort: Option<u64>,
        throttle: ProgressThrottle,
        force: bool,
    ) {
        let now = Instant::now();
        let phase_changed = match self.last_phase {
            None => true,
            Some(last_phase) => last_phase != phase,
        };
        let file_changed = match (current_file, self.last_heartbeat_file.as_deref()) {
            (Some(file), Some(last_file)) => file != last_file,
            (Some(_), None) => true,
            _ => false,
        };
        if phase_changed {
            self.last_heartbeat = None;
            self.last_heartbeat_file = None;
        }

        let should_emit = if force || phase_changed || file_changed {
            true
        } else {
            match throttle {
                ProgressThrottle::Standard => match self.last_emit {
                    None => true,
                    Some(last) => now.duration_since(last) >= THROTTLE_INTERVAL,
                },
                ProgressThrottle::Heartbeat => match self.last_heartbeat {
                    None => true,
                    Some(last) => now.duration_since(last) >= HEARTBEAT_INTERVAL,
                },
            }
        };

        if should_emit {
            let msg = ProgressMessage::Progress {
                phase,
                files_done,
                files_total,
                current_file: current_file.map(str::to_string),
                current_bytes_done,
                current_bytes_total,
                bytes_done_total_best_effort,
            };
            self.sink.emit(msg);
            match throttle {
                ProgressThrottle::Standard => {
                    self.last_emit = Some(now);
                }
                ProgressThrottle::Heartbeat => {
                    self.last_heartbeat = Some(now);
                    self.last_heartbeat_file = current_file.map(str::to_string);
                }
            }
            self.last_phase = Some(phase);
        }
    }

    pub fn emit_done(&mut self, bytes_written: u64, file_count: u64, scan_ms: u128, zip_ms: u128) {
        let msg = ProgressMessage::Done {
            bytes_written,
            file_count,
            scan_ms,
            zip_ms,
        };
        self.sink.emit(msg);
        self.last_emit = None;
        self.last_phase = None;
        self.last_heartbeat = None;
        self.last_heartbeat_file = None;
    }
}

enum ProgressThrottle {
    Standard,
    Heartbeat,
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
            current_file: None,
            current_bytes_done: None,
            current_bytes_total: None,
            bytes_done_total_best_effort: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"progress\""));
        assert!(json.contains("\"phase\":\"scanning\""));
    }
}
