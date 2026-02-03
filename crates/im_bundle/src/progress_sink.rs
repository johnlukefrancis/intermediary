// Path: crates/im_bundle/src/progress_sink.rs
// Description: Progress sink interfaces for bundle build reporting

use std::io::{self, Write};

use serde::Serialize;

use crate::progress::ProgressMessage;

pub trait ProgressSink: Send + Sync {
    fn emit(&self, message: ProgressMessage);
}

pub struct StdoutProgressSink;

impl StdoutProgressSink {
    pub fn new() -> Self {
        Self
    }
}

impl ProgressSink for StdoutProgressSink {
    fn emit(&self, message: ProgressMessage) {
        emit_json(&message);
    }
}

pub struct CallbackProgressSink<F>
where
    F: Fn(ProgressMessage) + Send + Sync,
{
    callback: F,
}

impl<F> CallbackProgressSink<F>
where
    F: Fn(ProgressMessage) + Send + Sync,
{
    pub fn new(callback: F) -> Self {
        Self { callback }
    }
}

impl<F> ProgressSink for CallbackProgressSink<F>
where
    F: Fn(ProgressMessage) + Send + Sync,
{
    fn emit(&self, message: ProgressMessage) {
        (self.callback)(message);
    }
}

fn emit_json<T: Serialize>(msg: &T) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = serde_json::to_writer(&mut handle, msg);
    let _ = handle.write_all(b"\n");
    let _ = handle.flush();
}
