// Path: crates/im_bundle/src/git_capture/command_child.rs
// Description: Stream worker threads, bounded pipe readers, and exit-status helpers for the Git runner

use std::io::{self, Read};
use std::process::ExitStatus;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

/// A stream thread whose result arrives on a channel, so every wait carries a
/// bound instead of joining a thread that a surviving descendant (holding the
/// pipe's write end) could keep alive indefinitely. A worker that has not
/// delivered by its deadline stays `Pending` and can be waited on again, which
/// is what lets the runner kill the pipe holder and then join.
pub(super) struct Worker<T> {
    receiver: mpsc::Receiver<T>,
    state: State<T>,
}

enum State<T> {
    /// Still reading; the deadline expired before it finished.
    Pending,
    Delivered(T),
    /// The thread ended without sending: nothing about the stream is known.
    Lost,
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
    Worker {
        receiver,
        state: State::Pending,
    }
}

impl<T> Worker<T> {
    /// Waits until `deadline` unless the worker already settled.
    fn wait(&mut self, deadline: Instant) {
        if !matches!(self.state, State::Pending) {
            return;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        self.state = match self.receiver.recv_timeout(remaining) {
            Ok(value) => State::Delivered(value),
            Err(mpsc::RecvTimeoutError::Timeout) => State::Pending,
            Err(mpsc::RecvTimeoutError::Disconnected) => State::Lost,
        };
    }

    /// `false` while the worker is still blocked on its pipe.
    fn settled(&self) -> bool {
        !matches!(self.state, State::Pending)
    }

    fn delivered(self) -> Option<T> {
        match self.state {
            State::Delivered(value) => Some(value),
            State::Pending | State::Lost => None,
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

/// What one stream delivered.
pub(super) enum StreamOutcome {
    /// The pipe reached EOF: the bytes kept, and whether the limit cut them off.
    Read(Vec<u8>, bool),
    /// The read failed or the worker was lost; the stream is not trustworthy.
    Failed,
    /// Still blocked when the runner gave up: the reader is detached and the
    /// bytes it had read are gone with it.
    Detached,
}

impl StreamOutcome {
    /// The bytes for a caller that only reports what Git managed to say.
    pub(super) fn bytes(self) -> Vec<u8> {
        match self {
            StreamOutcome::Read(bytes, _) => bytes,
            StreamOutcome::Failed | StreamOutcome::Detached => Vec::new(),
        }
    }
}

/// What the streams delivered once the runner stopped waiting on them.
pub(super) struct Collected {
    pub(super) stdout: StreamOutcome,
    pub(super) stderr: StreamOutcome,
    pub(super) stdin_failed: bool,
}

impl Streams {
    /// Waits for every worker until `deadline`; `false` when one is still
    /// blocked on its pipe, which means a descendant of the child holds it.
    pub(super) fn wait_all(&mut self, deadline: Instant) -> bool {
        if let Some(stdin) = self.stdin.as_mut() {
            stdin.wait(deadline);
        }
        self.stdout.wait(deadline);
        self.stderr.wait(deadline);
        self.stdout.settled()
            && self.stderr.settled()
            && self.stdin.as_ref().is_none_or(Worker::settled)
    }

    pub(super) fn into_collected(self) -> Collected {
        Collected {
            stdout: outcome(self.stdout),
            stderr: outcome(self.stderr),
            stdin_failed: self
                .stdin
                .and_then(Worker::delivered)
                .is_some_and(|result| result.is_err()),
        }
    }
}

fn outcome(worker: Worker<StreamRead>) -> StreamOutcome {
    let pending = !worker.settled();
    match worker.delivered() {
        Some(Ok((bytes, truncated))) => StreamOutcome::Read(bytes, truncated),
        Some(Err(_)) => StreamOutcome::Failed,
        None if pending => StreamOutcome::Detached,
        None => StreamOutcome::Failed,
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
