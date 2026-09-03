// Path: crates/im_agent/src/source_control/tests_support.rs
// Description: Real-git tempdir fixtures shared by the source-control tests

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

use crate::error::AgentError;
use crate::protocol::{
    SourceControlActionPayload, SourceControlChange, SourceControlDiscardTarget, SourceControlEntry,
    SourceControlEntryArea, SourceControlScope, SourceControlStatus, SourceControlWorktreeStamp,
};

use crate::source_control::status::stamp::stamp_of;
use crate::source_control::{
    run_source_control_action, source_control_status, SourceControlActionOutcome,
    SourceControlLocks,
};

/// Empty repository on branch `main` with a local identity; no commits yet.
pub(super) fn init_repo() -> (TempDir, PathBuf) {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().join("repo");
    std::fs::create_dir_all(&root).expect("repo dir");
    git(&root, &["init", "-q", "-b", "main"]);
    git(&root, &["config", "user.email", "source-control@example.test"]);
    git(&root, &["config", "user.name", "Source Control Test"]);
    (temp, root)
}

/// Repository with one commit containing `base.txt` (`base\n`).
pub(super) fn init_repo_with_commit() -> (TempDir, PathBuf) {
    let (temp, root) = init_repo();
    write(&root, "base.txt", b"base\n");
    git(&root, &["add", "."]);
    git(&root, &["commit", "-qm", "baseline"]);
    (temp, root)
}

pub(super) fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Runs Git and reports success instead of asserting it (conflicting merges).
pub(super) fn git_succeeds(root: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("run git")
        .status
        .success()
}

pub(super) fn git_stdout(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("run git");
    assert!(output.status.success(), "git {args:?} failed");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

pub(super) fn write(root: &Path, relative: &str, bytes: &[u8]) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent dir");
    }
    std::fs::write(path, bytes).expect("write file");
}

pub(super) fn read(root: &Path, relative: &str) -> Vec<u8> {
    std::fs::read(root.join(relative)).expect("read file")
}

pub(super) async fn status(root: &Path) -> SourceControlStatus {
    status_with(&SourceControlLocks::new(), root).await
}

pub(super) async fn status_with(locks: &SourceControlLocks, root: &Path) -> SourceControlStatus {
    source_control_status(root, None, locks).await.expect("status")
}

pub(super) async fn act(root: &Path, action: SourceControlActionPayload) -> SourceControlActionOutcome {
    try_act(root, action).await.expect("action")
}

pub(super) async fn try_act(
    root: &Path,
    action: SourceControlActionPayload,
) -> Result<SourceControlActionOutcome, AgentError> {
    try_act_with(&SourceControlLocks::new(), root, action).await
}

pub(super) async fn try_act_with(
    locks: &SourceControlLocks,
    root: &Path,
    action: SourceControlActionPayload,
) -> Result<SourceControlActionOutcome, AgentError> {
    run_source_control_action(locks, root, action).await
}

/// A commit that names the snapshot the caller has just read, the way the UI
/// sends the identity it displayed.
pub(super) async fn commit_now(root: &Path, message: &str) -> SourceControlActionPayload {
    commit_for(&status(root).await, message)
}

/// The commit the UI would send for a status it is already holding.
pub(super) fn commit_for(reviewed: &SourceControlStatus, message: &str) -> SourceControlActionPayload {
    SourceControlActionPayload::Commit {
        message: message.to_string(),
        expected_snapshot_id: reviewed.snapshot_id.clone(),
    }
}

/// Installs an executable Git hook. The body is written to a sibling path and
/// renamed into place: Git forks while this process may still hold a write
/// handle open, and exec'ing a file another descriptor has open for writing
/// fails with ETXTBSY. Renaming publishes a file no descriptor of ours points
/// at, so the hook is either absent or executable, never busy.
#[cfg(unix)]
pub(super) fn write_hook(root: &Path, name: &str, body: &str) {
    use std::os::unix::fs::PermissionsExt;

    let hooks = root.join(".git/hooks");
    std::fs::create_dir_all(&hooks).expect("hooks dir");
    let staging = hooks.join(format!("{name}.staging"));
    std::fs::write(&staging, format!("#!/bin/sh\n{body}")).expect("hook body");
    std::fs::set_permissions(&staging, std::fs::Permissions::from_mode(0o755)).expect("hook mode");
    std::fs::rename(&staging, hooks.join(name)).expect("publish hook");
}

/// A discard whose targets carry the stamps on disk right now (or
/// `expectedMissing: true` when nothing is there), the way the UI returns
/// what it displayed.
pub(super) fn discard_now(root: &Path, paths: &[&str]) -> SourceControlActionPayload {
    SourceControlActionPayload::Discard {
        targets: paths.iter().map(|path| target_now(root, path)).collect(),
    }
}

/// Mirrors what the real UI sends for one target: the stamp when the file
/// exists, `expectedMissing: true` only when it is genuinely absent, and
/// neither when the path exists but is not a regular file (a directory —
/// never a status entry to begin with, so never worth a claim).
pub(super) fn target_now(root: &Path, path: &str) -> SourceControlDiscardTarget {
    let disk = stamp_of(&root.join(path));
    match (disk.stamp, disk.missing) {
        (Some(stamp), _) => target(path, Some(stamp)),
        (None, true) => missing_target(path),
        (None, false) => target(path, None),
    }
}

pub(super) fn target(
    path: &str,
    expected_stamp: Option<SourceControlWorktreeStamp>,
) -> SourceControlDiscardTarget {
    SourceControlDiscardTarget {
        path: path.to_string(),
        expected_stamp,
        expected_missing: false,
    }
}

pub(super) fn missing_target(path: &str) -> SourceControlDiscardTarget {
    SourceControlDiscardTarget {
        path: path.to_string(),
        expected_stamp: None,
        expected_missing: true,
    }
}

/// The discard quarantine directories under one git dir. Absent directory =
/// no operation ever ran here, which is a legitimate answer, not a failure.
pub(super) fn quarantine_entries(git_dir: &Path) -> Vec<PathBuf> {
    let dir = git_dir.join("intermediary-discard");
    match std::fs::read_dir(&dir) {
        Ok(read) => read.map(|entry| entry.expect("read dir").path()).collect(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => panic!("read {}: {error}", dir.display()),
    }
}

/// The one quarantine directory a single-claiming-target action left behind.
pub(super) fn only_operation(git_dir: &Path) -> PathBuf {
    let mut found = quarantine_entries(git_dir);
    assert_eq!(
        found.len(),
        1,
        "one claiming target owns exactly one quarantine directory"
    );
    found.remove(0)
}

pub(super) fn text(path: &Path) -> String {
    String::from_utf8(std::fs::read(path).expect("read file")).expect("utf-8")
}

pub(super) fn disk_stamp(root: &Path, path: &str) -> Option<SourceControlWorktreeStamp> {
    stamp_of(&root.join(path)).stamp
}

pub(super) fn paths_scope(paths: &[&str]) -> SourceControlScope {
    SourceControlScope::Paths {
        paths: paths.iter().map(|path| path.to_string()).collect(),
    }
}

pub(super) fn entry(
    path: &str,
    area: SourceControlEntryArea,
    change: SourceControlChange,
) -> SourceControlEntry {
    SourceControlEntry {
        path: path.to_string(),
        original_path: None,
        area,
        change,
        worktree_stamp: None,
        worktree_missing: false,
    }
}

pub(super) fn renamed_entry(path: &str, original: &str) -> SourceControlEntry {
    SourceControlEntry {
        path: path.to_string(),
        original_path: Some(original.to_string()),
        area: SourceControlEntryArea::Index,
        change: SourceControlChange::Renamed,
        worktree_stamp: None,
        worktree_missing: false,
    }
}

/// Worktree and conflict entries carry the file's stamp and its on-disk
/// presence; comparisons about which paths are listed drop both, and either
/// is asserted separately where a test cares about it.
pub(super) fn stripped(entries: &[SourceControlEntry]) -> Vec<SourceControlEntry> {
    entries
        .iter()
        .map(|entry| SourceControlEntry {
            worktree_stamp: None,
            worktree_missing: false,
            ..entry.clone()
        })
        .collect()
}

pub(super) fn paths_of(entries: &[SourceControlEntry]) -> Vec<&str> {
    entries.iter().map(|entry| entry.path.as_str()).collect()
}
