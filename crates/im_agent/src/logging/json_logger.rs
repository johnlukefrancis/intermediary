// Path: crates/im_agent/src/logging/json_logger.rs
// Description: JSONL logger that writes to agent_latest.log and stdout/stderr

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use chrono::Utc;
use serde::Serialize;
use serde_json::Value;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::error::AgentError;

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
    sender: UnboundedSender<LogEntry>,
}

#[derive(Clone)]
pub struct Logger {
    inner: Arc<LoggerInner>,
}

pub struct LogConfig {
    pub log_dir: PathBuf,
    pub min_level: LogLevel,
}

impl Logger {
    pub async fn init(config: LogConfig) -> Result<Self, AgentError> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let logger = Logger {
            inner: Arc::new(LoggerInner {
                min_level: AtomicU8::new(config.min_level.priority()),
                sender,
            }),
        };

        let log_file = config.log_dir.join("agent_latest.log");
        tokio::spawn(run_writer(config.log_dir, log_file, receiver));

        Ok(logger)
    }

    pub fn set_level(&self, level: LogLevel) {
        self.inner.min_level.store(level.priority(), Ordering::Relaxed);
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
            let fallback = format!(
                "{{\"level\":\"error\",\"msg\":\"Log channel closed\"}}"
            );
            eprintln!("{fallback}");
        }
    }
}

async fn run_writer(log_dir: PathBuf, log_file: PathBuf, mut receiver: UnboundedReceiver<LogEntry>) {
    if let Err(err) = fs::create_dir_all(&log_dir).await {
        let fallback = format!(
            "{{\"level\":\"error\",\"msg\":\"Failed to create log dir\",\"error\":\"{}\"}}",
            err
        );
        eprintln!("{fallback}");
        return;
    }

    let mut file = match OpenOptions::new().create(true).append(true).open(&log_file).await {
        Ok(file) => file,
        Err(err) => {
            let fallback = format!(
                "{{\"level\":\"error\",\"msg\":\"Failed to open log file\",\"error\":\"{}\"}}",
                err
            );
            eprintln!("{fallback}");
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
                eprintln!("{fallback}");
                continue;
            }
        };

        if let Err(err) = append_line(&mut file, &line).await {
            let fallback = format!(
                "{{\"level\":\"error\",\"msg\":\"Failed to write log entry\",\"error\":\"{}\"}}",
                err
            );
            eprintln!("{fallback}");
        }

        if entry.level == "error" {
            eprintln!("{line}");
        } else {
            println!("{line}");
        }
    }
}

async fn append_line(file: &mut tokio::fs::File, line: &str) -> Result<(), std::io::Error> {
    file.write_all(line.as_bytes()).await?;
    file.write_all(b"\n").await?;
    file.flush().await
}

pub fn resolve_log_dir(raw: Option<String>) -> PathBuf {
    match raw {
        Some(dir) if !dir.trim().is_empty() => PathBuf::from(dir),
        _ => default_log_dir(),
    }
}

fn default_log_dir() -> PathBuf {
    match std::env::current_dir() {
        Ok(dir) => dir.join("logs"),
        Err(_) => Path::new("logs").to_path_buf(),
    }
}
