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
use super::command_tree_owner::owner_for;

pub use super::command_failure::{GitCommandFailure, GitCommandFailureKind};

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

/// How a still-running Git child is stopped, and the marker for a mutation:
/// `Graceful` commands take Git's mandatory locks (`add`, `reset`, `commit`,
/// `push`, `pull`), so they are asked to end before the tree is killed — and
/// they refuse to run at all without an owner for that tree.
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

/// The child's fate after the wait loop: exited by itself (its streams get the
/// bounded post-exit drain), stopped on timeout (a bounded wait), or never
/// waitable at all, in which case nothing about it is trustworthy.
enum Exit {
    Completed(ExitStatus),
    TimedOut,
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
    // The tree that owns every descendant this command starts, created before
    // the spawn and dropped once the drain below is finished. Dropping it
    // terminates nothing: the tree dies only where this file asks for it — a
    // forced stop below, an expired post-exit drain, or shutdown finalization.
    // A mutation that cannot be given one is refused here, before the spawn.
    let mut tree = match owner_for(kill_policy) {
        Ok(tree) => tree,
        Err(failure) => return Ok(Err(failure)),
    };
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
    if let Err(failure) = tree.attach(&child, kill_policy) {
        stop_child(&mut child, KillPolicy::Immediate, &tree);
        return Ok(Err(failure));
    }
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
    // line can name the process the descendants came from.
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
