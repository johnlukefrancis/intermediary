// Path: crates/im_agent/src/source_control/tests_support.rs
// Description: Real-git tempdir fixtures shared by the source-control tests

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

use crate::error::AgentError;
use crate::protocol::{
    SourceControlActionPayload, SourceControlChange, SourceControlEntry, SourceControlEntryArea,
    SourceControlScope, SourceControlStatus,
};

use super::{
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
    source_control_status(root, None).await.expect("status")
}

pub(super) async fn act(root: &Path, action: SourceControlActionPayload) -> SourceControlActionOutcome {
    try_act(root, action).await.expect("action")
}

pub(super) async fn try_act(
    root: &Path,
    action: SourceControlActionPayload,
) -> Result<SourceControlActionOutcome, AgentError> {
    let locks = SourceControlLocks::new();
    run_source_control_action(&locks, "repo", root, action).await
}

pub(super) fn paths_scope(paths: &[&str]) -> SourceControlScope {
    SourceControlScope::Paths {
        paths: paths.iter().map(|path| path.to_string()).collect(),
    }
}

pub(super) fn strings(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|path| path.to_string()).collect()
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
    }
}

pub(super) fn renamed_entry(path: &str, original: &str) -> SourceControlEntry {
    SourceControlEntry {
        path: path.to_string(),
        original_path: Some(original.to_string()),
        area: SourceControlEntryArea::Index,
        change: SourceControlChange::Renamed,
    }
}

pub(super) fn paths_of(entries: &[SourceControlEntry]) -> Vec<&str> {
    entries.iter().map(|entry| entry.path.as_str()).collect()
}
