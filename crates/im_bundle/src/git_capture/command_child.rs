// Path: crates/im_bundle/src/git_capture/command_child.rs
// Description: Stream worker threads, bounded pipe readers, and exit-status helpers for the Git runner

use std::io::{self, Read};
use std::process::ExitStatus;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

/// A stream thread whose result arrives on a channel, so a forced stop can
/// wait with a bound instead of joining a thread that a surviving grandchild
/// (holding the pipe's write end) could keep alive indefinitely.
pub(super) struct Worker<T> {
    receiver: mpsc::Receiver<T>,
}

pub(super) fn spawn_worker<T, F>(work: F) -> Worker<T>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(work());
    });
    Worker { receiver }
}

/// How long to wait for the stream workers once the child has been reaped.
#[derive(Clone, Copy)]
pub(super) enum Wait {
    /// The child exited by itself: its pipes close as its last holder exits.
    UntilDone,
    /// The child was stopped: wait until the deadline, then detach.
    Until(Instant),
}

impl<T> Worker<T> {
    /// `None` when the worker panicked or, under a deadline, has not finished;
    /// a detached worker exits on its own when the pipe finally closes.
    fn wait(self, wait: Wait) -> Option<T> {
        match wait {
            Wait::UntilDone => self.receiver.recv().ok(),
            Wait::Until(deadline) => self
                .receiver
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .ok(),
        }
    }
}

pub(super) type StreamRead = io::Result<(Vec<u8>, bool)>;

/// The three stream workers of one child.
pub(super) struct Streams {
    pub(super) stdout: Worker<StreamRead>,
    pub(super) stderr: Worker<StreamRead>,
    pub(super) stdin: Option<Worker<io::Result<()>>>,
}

/// What the streams delivered: `None` for a stream whose read failed or whose
/// worker was detached.
pub(super) struct Collected {
    pub(super) stdout: Option<(Vec<u8>, bool)>,
    pub(super) stderr: Option<(Vec<u8>, bool)>,
    pub(super) stdin_failed: bool,
}

impl Streams {
    pub(super) fn collect(self, wait: Wait) -> Collected {
        let stdin_failed = self
            .stdin
            .and_then(|writer| writer.wait(wait))
            .is_some_and(|result| result.is_err());
        Collected {
            stdout: self.stdout.wait(wait).and_then(Result::ok),
            stderr: self.stderr.wait(wait).and_then(Result::ok),
            stdin_failed,
        }
    }
}

pub(super) fn read_bounded<R: Read>(mut reader: R, limit: usize) -> StreamRead {
    let mut captured = Vec::with_capacity(limit.min(64 * 1024));
    let mut truncated = false;
    let mut buffer = [0u8; 16 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let remaining = limit.saturating_sub(captured.len());
        let keep = remaining.min(read);
        captured.extend_from_slice(&buffer[..keep]);
        truncated |= keep < read;
    }
    Ok((captured, truncated))
}

pub(super) fn accepted_exit_status(status: ExitStatus, accepted_nonzero_codes: &[i32]) -> bool {
    status
        .code()
        .is_some_and(|code| accepted_nonzero_codes.contains(&code))
}

pub(super) fn contains_not_repository(stderr: &[u8]) -> bool {
    let lowered = String::from_utf8_lossy(stderr).to_ascii_lowercase();
    lowered.contains("not a git repository") || lowered.contains("not a git work tree")
}
