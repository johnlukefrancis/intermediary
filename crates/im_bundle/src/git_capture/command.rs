// Path: crates/im_bundle/src/git_capture/command.rs
// Description: Bounded, cancellable Git subprocess execution shared by bundle evidence and source control

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::cancel::BundleCancelToken;
use crate::error::{BundleError, Result};

use super::command_child::{
    accepted_exit_status, contains_not_repository, read_bounded, spawn_worker, Collected,
    StreamOutcome, Streams,
};
use super::command_drain::{drain_after_exit, drain_after_forced_stop};
use super::command_stop::stop_child;
use super::command_tree::GitProcessTree;

const POLL_INTERVAL: Duration = Duration::from_millis(10);
const STDERR_LIMIT: usize = 64 * 1024;

/// Captured output of a completed Git command. `stderr` is bounded and kept so
/// callers can surface Git's own explanation of a failure; `exit_code` is zero
/// unless the caller accepted the non-zero code Git returned.
#[derive(Debug)]
pub struct GitCommandOutput {
    pub stdout: Vec<u8>,
    pub stdout_truncated: bool,
    pub stderr: Vec<u8>,
    pub exit_code: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitCommandFailureKind {
    MissingExecutable,
    TimedOut,
    SpawnFailed,
    InputWriteFailed,
    OutputReadFailed,
    NotGitRepository,
    NonZeroExit,
}

/// Why a Git command did not produce usable output, plus whatever Git wrote on
/// both streams before failing. Bounded like successful output.
#[derive(Debug, Clone)]
pub struct GitCommandFailure {
    pub kind: GitCommandFailureKind,
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl GitCommandFailure {
    fn bare(kind: GitCommandFailureKind) -> Self {
        Self {
            kind,
            exit_code: None,
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }

    fn from_streams(kind: GitCommandFailureKind, output: Collected) -> Self {
        Self {
            kind,
            exit_code: None,
            stdout: output.stdout.bytes(),
            stderr: output.stderr.bytes(),
        }
    }

    /// Git's explanation, preferring stderr and falling back to stdout
    /// (`git commit` reports "nothing to commit" on stdout).
    pub fn message(&self) -> String {
        let stderr = String::from_utf8_lossy(&self.stderr);
        let trimmed = stderr.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
        String::from_utf8_lossy(&self.stdout).trim().to_string()
    }
}

/// How a still-running Git child is stopped on timeout or cancellation.
/// `Graceful` is required for commands that take Git's mandatory locks
/// (`add`, `reset`, `commit`, `push`, `pull`): an immediate kill bypasses Git's
/// lockfile cleanup and can leave `.git/index.lock` behind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillPolicy {
    Immediate,
    Graceful,
}

pub fn run_git(
    executable: &Path,
    repo_root: &Path,
    args: &[OsString],
    stdout_limit: usize,
    timeout: Duration,
    cancel_token: Option<&BundleCancelToken>,
) -> Result<std::result::Result<GitCommandOutput, GitCommandFailure>> {
    run_git_with_input(
        executable,
        repo_root,
        args,
        None,
        &[],
        stdout_limit,
        timeout,
        cancel_token,
        KillPolicy::Immediate,
    )
}

/// The child's fate after the wait loop.
enum Exit {
    /// Git exited by itself; its streams get the bounded post-exit drain.
    Completed(ExitStatus),
    /// Git was stopped on timeout; the streams get a bounded wait.
    TimedOut,
    /// The child could not be waited on; nothing about it is trustworthy.
    WaitFailed,
}

#[allow(clippy::too_many_arguments)]
pub fn run_git_with_input(
    executable: &Path,
    repo_root: &Path,
    args: &[OsString],
    stdin: Option<Vec<u8>>,
    accepted_nonzero_codes: &[i32],
    stdout_limit: usize,
    timeout: Duration,
    cancel_token: Option<&BundleCancelToken>,
    kill_policy: KillPolicy,
) -> Result<std::result::Result<GitCommandOutput, GitCommandFailure>> {
    let mut command = Command::new(executable);
    command
        .arg("--no-pager")
        .args(args)
        .current_dir(repo_root)
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("GIT_PAGER", "cat")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // The tree that owns every descendant this command starts. It is created
    // before the spawn (the Windows job object has to exist first), joined to
    // the child immediately after it, and dropped once the drain below is
    // finished. Dropping it terminates nothing on either platform: the tree
    // dies only where this file asks for it — a forced stop below, an expired
    // post-exit drain, or the agent's shutdown finalization.
    let mut tree = GitProcessTree::create();
    tree.prepare(&mut command);

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(Err(GitCommandFailure::bare(
                GitCommandFailureKind::MissingExecutable,
            )));
        }
        Err(_) => {
            return Ok(Err(GitCommandFailure::bare(
                GitCommandFailureKind::SpawnFailed,
            )))
        }
    };
    tree.attach(&child);
    let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
        stop_child(&mut child, KillPolicy::Immediate, &tree);
        return Ok(Err(GitCommandFailure::bare(
            GitCommandFailureKind::OutputReadFailed,
        )));
    };
    let stdout = spawn_worker(move || read_bounded(stdout, stdout_limit));
    let stderr = spawn_worker(move || read_bounded(stderr, STDERR_LIMIT));
    let stdin = match stdin {
        Some(input) => {
            let Some(mut child_stdin) = child.stdin.take() else {
                stop_child(&mut child, KillPolicy::Immediate, &tree);
                return Ok(Err(GitCommandFailure::bare(
                    GitCommandFailureKind::InputWriteFailed,
                )));
            };
            Some(spawn_worker(move || child_stdin.write_all(&input)))
        }
        None => None,
    };
    let streams = Streams {
        stdout,
        stderr,
        stdin,
    };
    // Kept before the wait loop reaps the child, purely so a stuck-pipe log
    // line can name the process the descendants came from; the tree above is
    // what actually still holds them.
    let child_pid = child.id();
    let started = Instant::now();

    let exit = loop {
        if cancel_token.is_some_and(BundleCancelToken::is_cancelled) {
            stop_child(&mut child, kill_policy, &tree);
            drain_after_forced_stop(streams);
            return Err(BundleError::Cancelled);
        }
        match child.try_wait() {
            Ok(Some(status)) => break Exit::Completed(status),
            Ok(None) if started.elapsed() < timeout => thread::sleep(POLL_INTERVAL),
            Ok(None) => {
                stop_child(&mut child, kill_policy, &tree);
                break Exit::TimedOut;
            }
            Err(_) => {
                stop_child(&mut child, KillPolicy::Immediate, &tree);
                break Exit::WaitFailed;
            }
        }
    };

    let status = match exit {
        Exit::Completed(status) => status,
        Exit::TimedOut => {
            let output = drain_after_forced_stop(streams);
            return Ok(Err(GitCommandFailure::from_streams(
                GitCommandFailureKind::TimedOut,
                output,
            )));
        }
        Exit::WaitFailed => {
            drain_after_forced_stop(streams);
            return Ok(Err(GitCommandFailure::bare(
                GitCommandFailureKind::OutputReadFailed,
            )));
        }
    };
    let Collected {
        stdout,
        stderr,
        stdin_failed,
    } = drain_after_exit(streams, &tree, child_pid);
    let read_failed = || {
        Ok(Err(GitCommandFailure::bare(
            GitCommandFailureKind::OutputReadFailed,
        )))
    };
    // A detached reader keeps nothing it had read, so its stream is reported
    // as empty and truncated: incomplete output, never silently complete.
    let (stdout, stdout_truncated) = match stdout {
        StreamOutcome::Read(bytes, truncated) => (bytes, truncated),
        StreamOutcome::Detached => (Vec::new(), true),
        StreamOutcome::Failed => return read_failed(),
    };
    let stderr = match stderr {
        StreamOutcome::Read(bytes, _) => bytes,
        StreamOutcome::Detached => Vec::new(),
        StreamOutcome::Failed => return read_failed(),
    };
    let failed = |kind| GitCommandFailure {
        kind,
        exit_code: status.code(),
        stdout: stdout.clone(),
        stderr: stderr.clone(),
    };
    if stdin_failed {
        return Ok(Err(failed(GitCommandFailureKind::InputWriteFailed)));
    }
    if !status.success() {
        if contains_not_repository(&stderr) {
            return Ok(Err(failed(GitCommandFailureKind::NotGitRepository)));
        }
        if !accepted_exit_status(status, accepted_nonzero_codes) {
            return Ok(Err(failed(GitCommandFailureKind::NonZeroExit)));
        }
    }

    Ok(Ok(GitCommandOutput {
        stdout,
        stdout_truncated,
        stderr,
        exit_code: status.code().unwrap_or(0),
    }))
}
