// Path: src-tauri/src/lib/terminal/output_sink.rs
// Description: Non-blocking detachable owner of a terminal session's bounded webview output channel

use super::flow_gate::FlowGate;
use super::frames::TerminalExitFrame;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::ipc::{Channel, InvokeResponseBody};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishOutcome {
    Published,
    Detached,
}

pub struct OutputSink {
    channel: Channel<InvokeResponseBody>,
    attached: AtomicBool,
}

impl OutputSink {
    pub fn new(channel: Channel<InvokeResponseBody>) -> Self {
        Self {
            channel,
            attached: AtomicBool::new(true),
        }
    }

    /// A close never waits behind publication: an atomic detach prevents every
    /// later frame from beginning, while a frame that already observed the old
    /// state may finish its one bounded send.
    pub fn detach(&self) {
        self.attached.store(false, Ordering::Release);
    }

    pub fn publish(&self, bytes: &[u8], gate: &FlowGate) -> Result<PublishOutcome, String> {
        if !self.attached.load(Ordering::Acquire) {
            return Ok(PublishOutcome::Detached);
        }
        gate.charge(bytes.len() as u64)?;
        self.channel
            .send(InvokeResponseBody::Raw(bytes.to_vec()))
            .map(|()| PublishOutcome::Published)
            .map_err(|err| {
                self.detach();
                format!("Terminal output channel send failed: {err}")
            })
    }

    pub fn publish_exit(&self, frame: &TerminalExitFrame) -> Result<PublishOutcome, String> {
        if !self.attached.load(Ordering::Acquire) {
            return Ok(PublishOutcome::Detached);
        }
        let json = serde_json::to_string(frame)
            .map_err(|err| format!("Failed to encode terminal exit frame: {err}"))?;
        self.channel
            .send(InvokeResponseBody::Json(json))
            .map(|()| PublishOutcome::Published)
            .map_err(|err| {
                self.detach();
                format!("Terminal exit channel send failed: {err}")
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{OutputSink, PublishOutcome};
    use crate::terminal::flow_gate::FlowGate;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};
    use std::thread;
    use tauri::ipc::Channel;

    #[test]
    fn detach_allows_only_the_already_started_bounded_publish() {
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let sends = Arc::new(AtomicUsize::new(0));
        let channel = Channel::new({
            let entered = entered.clone();
            let release = release.clone();
            let sends = sends.clone();
            move |_| {
                sends.fetch_add(1, Ordering::SeqCst);
                entered.wait();
                release.wait();
                Ok(())
            }
        });
        let sink = Arc::new(OutputSink::new(channel));
        let gate = Arc::new(FlowGate::new());
        let publisher = {
            let sink = sink.clone();
            let gate = gate.clone();
            thread::spawn(move || sink.publish(&[1], &gate))
        };

        entered.wait();
        sink.detach();
        assert_eq!(sink.publish(&[2], &gate), Ok(PublishOutcome::Detached));
        release.wait();
        assert_eq!(
            publisher.join().expect("publisher"),
            Ok(PublishOutcome::Published)
        );
        assert_eq!(sends.load(Ordering::SeqCst), 1);
    }
}
