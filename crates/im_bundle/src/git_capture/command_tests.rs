// Path: crates/im_bundle/src/git_capture/command_tests.rs
// Description: Forced-stop tests for the bounded Git runner: process-group kill and detached stream readers

#![cfg(unix)]

use std::ffi::OsString;
use std::path::Path;
use std::time::{Duration, Instant};

use tempfile::tempdir;

use super::command::{run_git, GitCommandFailure, GitCommandFailureKind, GitCommandOutput};
use super::fake_git::write_fake_git;

const TIMEOUT: Duration = Duration::from_millis(100);

fn run_until_timeout(fake_git: &Path, repo: &Path) -> (GitCommandFailure, Duration) {
    let started = Instant::now();
    let result = run_git(
        fake_git,
        repo,
        &[OsString::from("status")],
        4096,
        TIMEOUT,
        None,
    )
    .expect("runner result");
    let failure = result.expect_err("the fake git never exits by itself");
    assert_eq!(failure.kind, GitCommandFailureKind::TimedOut);
    (failure, started.elapsed())
}

fn run_to_completion(fake_git: &Path, repo: &Path) -> (GitCommandOutput, Duration) {
    let started = Instant::now();
    let output = run_git(
        fake_git,
        repo,
        &[OsString::from("status")],
        4096,
        Duration::from_secs(30),
        None,
    )
    .expect("runner result")
    .expect("the fake git exits by itself");
    (output, started.elapsed())
}

fn read_pid(pid_file: &Path) -> libc::pid_t {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(pid) = std::fs::read_to_string(pid_file)
            .ok()
            .and_then(|text| text.trim().parse().ok())
        {
            return pid;
        }
        assert!(Instant::now() < deadline, "fake git never wrote its pid");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn group_is_gone(group: libc::pid_t) -> bool {
    // SAFETY: signal 0 delivers nothing; it only reports whether the group
    // still has members.
    unsafe { libc::kill(-group, 0) != 0 }
}

/// A grandchild inherits Git's stdout. Without the process-group kill the
/// runner sat on the reader until that grandchild exited on its own.
#[test]
fn timed_out_child_with_a_grandchild_on_stdout_returns_promptly_and_leaves_no_group() {
    let root = tempdir().expect("tempdir");
    let pid_file = root.path().join("pid");
    let script = format!(
        "echo $$ > \"{}\"\nsleep 30 & sleep 30\n",
        pid_file.display()
    );
    let fake_git = write_fake_git(root.path(), "holding-git", &script);

    let (_, elapsed) = run_until_timeout(&fake_git, root.path());
    assert!(elapsed < Duration::from_secs(2), "took {elapsed:?}");

    let group = read_pid(&pid_file);
    let deadline = Instant::now() + Duration::from_secs(2);
    while !group_is_gone(group) {
        assert!(
            Instant::now() < deadline,
            "process group {group} still has members"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// A grandchild that left the group (`setsid`) still holds stdout after the
/// kill; the readers are detached after the bounded wait instead of blocking
/// until that grandchild exits.
#[test]
fn escaped_grandchild_detaches_the_readers_after_the_bounded_wait() {
    if !Path::new("/usr/bin/setsid").exists() {
        return;
    }
    let root = tempdir().expect("tempdir");
    let fake_git = write_fake_git(
        root.path(),
        "escaping-git",
        "echo partial\n/usr/bin/setsid sleep 5 &\nsleep 30\n",
    );

    let (failure, elapsed) = run_until_timeout(&fake_git, root.path());
    assert!(
        elapsed >= Duration::from_secs(2) && elapsed < Duration::from_secs(4),
        "took {elapsed:?}"
    );
    assert!(
        failure.stdout.is_empty(),
        "detached reader delivers nothing"
    );
}

/// The direct Git child exits, but a descendant it backgrounded still holds
/// stdout. The drain is bounded: the group is killed, the reader joins, and
/// the caller gets Git's exit status and everything Git printed.
#[test]
fn exited_child_with_a_backgrounded_pipe_holder_returns_within_the_grace() {
    let root = tempdir().expect("tempdir");
    let fake_git = write_fake_git(
        root.path(),
        "hook-holding-git",
        "echo committed\nsleep 5 &\nexit 0\n",
    );

    let (output, elapsed) = run_to_completion(&fake_git, root.path());
    assert!(elapsed < Duration::from_secs(4), "took {elapsed:?}");
    assert_eq!(output.exit_code, 0);
    assert_eq!(output.stdout, b"committed\n");
    assert!(!output.stdout_truncated);
}

/// Nothing holds the pipes: the drain adds no wait at all.
#[test]
fn exited_child_without_a_pipe_holder_returns_immediately() {
    let root = tempdir().expect("tempdir");
    let fake_git = write_fake_git(root.path(), "quick-git", "echo quick\n");

    let (output, elapsed) = run_to_completion(&fake_git, root.path());
    assert!(elapsed < Duration::from_millis(500), "took {elapsed:?}");
    assert_eq!(output.stdout, b"quick\n");
}
