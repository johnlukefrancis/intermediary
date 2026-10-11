// Path: crates/im_agent/src/logging/json_logger.rs
// Description: JSONL logger that writes each agent's own size-rotated log file and optionally mirrors to stdout/stderr

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use chrono::Utc;
use serde::Serialize;
use serde_json::Value;
use tokio::fs;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::error::AgentError;

use super::log_file::{LogFile, LOG_ROTATE_BYTES};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }

    fn priority(self) -> u8 {
        match self {
            LogLevel::Debug => 0,
            LogLevel::Info => 1,
            LogLevel::Warn => 2,
            LogLevel::Error => 3,
        }
    }
}

impl std::str::FromStr for LogLevel {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.trim().to_lowercase().as_str() {
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Serialize)]
struct LogEntry {
    level: String,
    ts: String,
    msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

struct LoggerInner {
    min_level: AtomicU8,
    emit_stdio: bool,
    sender: UnboundedSender<LogEntry>,
}

#[derive(Clone)]
pub struct Logger {
    inner: Arc<LoggerInner>,
}

pub struct LogConfig {
    pub log_dir: PathBuf,
    /// Names `<stem>_latest.log`; each agent owns its own file so appends never interleave.
    pub file_stem: &'static str,
    pub min_level: LogLevel,
    pub emit_stdio: bool,
}

impl Logger {
    pub async fn init(config: LogConfig) -> Result<Self, AgentError> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let logger = Logger {
            inner: Arc::new(LoggerInner {
                min_level: AtomicU8::new(config.min_level.priority()),
                emit_stdio: config.emit_stdio,
                sender,
            }),
        };

        tokio::spawn(run_writer(
            config.log_dir,
            config.file_stem,
            receiver,
            config.emit_stdio,
        ));

        Ok(logger)
    }

    pub fn set_level(&self, level: LogLevel) {
        self.inner
            .min_level
            .store(level.priority(), Ordering::Relaxed);
    }

    pub fn debug(&self, msg: impl Into<String>, data: Option<Value>) {
        self.log(LogLevel::Debug, msg, data);
    }

    pub fn info(&self, msg: impl Into<String>, data: Option<Value>) {
        self.log(LogLevel::Info, msg, data);
    }

    pub fn warn(&self, msg: impl Into<String>, data: Option<Value>) {
        self.log(LogLevel::Warn, msg, data);
    }

    pub fn error(&self, msg: impl Into<String>, data: Option<Value>) {
        self.log(LogLevel::Error, msg, data);
    }

    fn log(&self, level: LogLevel, msg: impl Into<String>, data: Option<Value>) {
        if level.priority() < self.inner.min_level.load(Ordering::Relaxed) {
            return;
        }

        let entry = LogEntry {
            level: level.as_str().to_string(),
            ts: Utc::now().to_rfc3339(),
            msg: msg.into(),
            data,
        };

        if self.inner.sender.send(entry).is_err() {
            let fallback = format!("{{\"level\":\"error\",\"msg\":\"Log channel closed\"}}");
            if self.inner.emit_stdio {
                eprintln!("{fallback}");
            }
        }
    }
}

async fn run_writer(
    log_dir: PathBuf,
    file_stem: &'static str,
    mut receiver: UnboundedReceiver<LogEntry>,
    emit_stdio: bool,
) {
    if let Err(err) = fs::create_dir_all(&log_dir).await {
        let fallback = format!(
            "{{\"level\":\"error\",\"msg\":\"Failed to create log dir\",\"error\":\"{}\"}}",
            err
        );
        emit_stdio_fallback(emit_stdio, &fallback);
        return;
    }

    let mut file = match LogFile::open(&log_dir, file_stem, LOG_ROTATE_BYTES).await {
        Ok(file) => file,
        Err(err) => {
            let fallback = format!(
                "{{\"level\":\"error\",\"msg\":\"Failed to open log file\",\"error\":\"{}\"}}",
                err
            );
            emit_stdio_fallback(emit_stdio, &fallback);
            return;
        }
    };

    while let Some(entry) = receiver.recv().await {
        let line = match serde_json::to_string(&entry) {
            Ok(line) => line,
            Err(err) => {
                let fallback = format!(
                    "{{\"level\":\"error\",\"msg\":\"Failed to serialize log entry\",\"error\":\"{}\"}}",
                    err
                );
                emit_stdio_fallback(emit_stdio, &fallback);
                continue;
            }
        };

        if let Err(err) = file.append_line(&line).await {
            let fallback = format!(
                "{{\"level\":\"error\",\"msg\":\"Failed to write log entry\",\"error\":\"{}\"}}",
                err
            );
            emit_stdio_fallback(emit_stdio, &fallback);
        }

        match resolve_stdio_target(emit_stdio, &entry.level) {
            StdioTarget::Stdout => println!("{line}"),
            StdioTarget::Stderr => eprintln!("{line}"),
            StdioTarget::None => {}
        }
    }
}

fn emit_stdio_fallback(emit_stdio: bool, line: &str) {
    if emit_stdio {
        eprintln!("{line}");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StdioTarget {
    Stdout,
    Stderr,
    None,
}

fn resolve_stdio_target(emit_stdio: bool, level: &str) -> StdioTarget {
    if !emit_stdio {
        return StdioTarget::None;
    }
    if level == "error" {
        StdioTarget::Stderr
    } else {
        StdioTarget::Stdout
    }
}

pub fn resolve_log_dir(raw: Option<String>) -> PathBuf {
    match raw {
        Some(dir) if !dir.trim().is_empty() => PathBuf::from(dir),
        _ => default_log_dir(),
    }
}

pub fn resolve_stdio_logging(raw: Option<String>) -> bool {
    match raw {
        Some(value) => parse_stdio_logging(&value).unwrap_or(true),
        None => true,
    }
}

fn parse_stdio_logging(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn default_log_dir() -> PathBuf {
    match std::env::current_dir() {
        Ok(dir) => dir.join("logs"),
        Err(_) => Path::new("logs").to_path_buf(),
    }
}

#[cfg(test)]
#[path = "json_logger_tests.rs"]
mod tests;
