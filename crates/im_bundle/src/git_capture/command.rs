// Path: crates/im_bundle/src/git_capture/command.rs
// Description: Bounded, cancellable Git subprocess execution for bundle evidence

use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::cancel::BundleCancelToken;
use crate::error::{BundleError, Result};

const POLL_INTERVAL: Duration = Duration::from_millis(10);
const STDERR_LIMIT: usize = 64 * 1024;

#[derive(Debug)]
pub(crate) struct GitCommandOutput {
    pub(crate) stdout: Vec<u8>,
    pub(crate) stdout_truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GitCommandFailure {
    MissingExecutable,
    TimedOut,
    SpawnFailed,
    InputWriteFailed,
    OutputReadFailed,
    NotGitRepository,
    NonZeroExit,
}

pub(crate) fn run_git(
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
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn run_git_with_input(
    executable: &Path,
    repo_root: &Path,
    args: &[OsString],
    stdin: Option<Vec<u8>>,
    accepted_nonzero_codes: &[i32],
    stdout_limit: usize,
    timeout: Duration,
    cancel_token: Option<&BundleCancelToken>,
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

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(Err(GitCommandFailure::MissingExecutable));
        }
        Err(_) => return Ok(Err(GitCommandFailure::SpawnFailed)),
    };
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Ok(Err(GitCommandFailure::OutputReadFailed));
    };
    let Some(stderr) = child.stderr.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Ok(Err(GitCommandFailure::OutputReadFailed));
    };

    let stdout_reader = thread::spawn(move || read_bounded(stdout, stdout_limit));
    let stderr_reader = thread::spawn(move || read_bounded(stderr, STDERR_LIMIT));
    let stdin_writer = match stdin {
        Some(input) => {
            let Some(mut child_stdin) = child.stdin.take() else {
                let _ = child.kill();
                let _ = child.wait();
                join_reader(stdout_reader);
                join_reader(stderr_reader);
                return Ok(Err(GitCommandFailure::InputWriteFailed));
            };
            Some(thread::spawn(move || child_stdin.write_all(&input)))
        }
        None => None,
    };
    let started = Instant::now();
    let mut wait_failed = false;

    let status = loop {
        if cancel_token.is_some_and(BundleCancelToken::is_cancelled) {
            let _ = child.kill();
            let _ = child.wait();
            join_input_writer(stdin_writer);
            join_reader(stdout_reader);
            join_reader(stderr_reader);
            return Err(BundleError::Cancelled);
        }
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if started.elapsed() < timeout => thread::sleep(POLL_INTERVAL),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                wait_failed = true;
                break Some(failure_status());
            }
        }
    };

    let stdin_result = join_input_writer(stdin_writer);
    let stdout = join_reader(stdout_reader);
    let stderr = join_reader(stderr_reader);
    if wait_failed {
        return Ok(Err(GitCommandFailure::OutputReadFailed));
    }
    let ((stdout, stdout_truncated), (stderr, _stderr_truncated)) = match (stdout, stderr) {
        (Some(Ok(stdout)), Some(Ok(stderr))) => (stdout, stderr),
        _ => return Ok(Err(GitCommandFailure::OutputReadFailed)),
    };

    let Some(status) = status else {
        return Ok(Err(GitCommandFailure::TimedOut));
    };
    if stdin_result.is_some_and(|result| result.is_err()) {
        return Ok(Err(GitCommandFailure::InputWriteFailed));
    }
    if !status.success() {
        if contains_not_repository(&stderr) {
            return Ok(Err(GitCommandFailure::NotGitRepository));
        }
        if !accepted_exit_status(status, accepted_nonzero_codes) {
            return Ok(Err(GitCommandFailure::NonZeroExit));
        }
    }

    Ok(Ok(GitCommandOutput {
        stdout,
        stdout_truncated,
    }))
}

fn accepted_exit_status(status: ExitStatus, accepted_nonzero_codes: &[i32]) -> bool {
    status
        .code()
        .is_some_and(|code| accepted_nonzero_codes.contains(&code))
}

fn contains_not_repository(stderr: &[u8]) -> bool {
    let lowered = String::from_utf8_lossy(stderr).to_ascii_lowercase();
    lowered.contains("not a git repository") || lowered.contains("not a git work tree")
}

fn read_bounded<R: Read>(mut reader: R, limit: usize) -> io::Result<(Vec<u8>, bool)> {
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

fn join_reader<T>(handle: thread::JoinHandle<T>) -> Option<T> {
    handle.join().ok()
}

fn join_input_writer(handle: Option<thread::JoinHandle<io::Result<()>>>) -> Option<io::Result<()>> {
    handle.and_then(|handle| handle.join().ok())
}

#[cfg(unix)]
fn failure_status() -> std::process::ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(1 << 8)
}

#[cfg(windows)]
fn failure_status() -> std::process::ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(1)
}
