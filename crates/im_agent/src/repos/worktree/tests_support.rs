// Path: crates/im_agent/src/repos/worktree/tests_support.rs
// Description: Shared fixtures for the worktree action tests: a worktree, its files, and one call

use std::path::Path;

use tempfile::{tempdir, TempDir};

use crate::error::AgentError;
use crate::protocol::{ImportConflictPolicy, WorktreeAction};
use crate::source_control::SourceControlLocks;
use crate::staging::StageFileCancelToken;

use super::worktree_action;

/// A worktree with two folders, which is the smallest shape that lets an
/// entry move somewhere. No Git: move, copy and rename never speak to it.
pub(super) fn worktree() -> TempDir {
    let root = tempdir().expect("temp repo");
    std::fs::create_dir_all(root.path().join("app")).expect("app dir");
    std::fs::create_dir_all(root.path().join("docs")).expect("docs dir");
    root
}

pub(super) fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent dir");
    }
    std::fs::write(path, contents).expect("write file");
}

pub(super) fn read(root: &Path, relative: &str) -> String {
    std::fs::read_to_string(root.join(relative)).expect("read file")
}

pub(super) async fn act(root: &Path, action: WorktreeAction) -> Result<Vec<String>, AgentError> {
    worktree_action(
        root,
        &action,
        &SourceControlLocks::new(),
        &StageFileCancelToken::new(),
    )
    .await
}

pub(super) fn move_action(
    paths: &[&str],
    directory: &str,
    on_conflict: ImportConflictPolicy,
) -> WorktreeAction {
    WorktreeAction::Move {
        paths: owned(paths),
        directory: directory.to_string(),
        on_conflict,
    }
}

pub(super) fn copy_action(
    paths: &[&str],
    directory: &str,
    on_conflict: ImportConflictPolicy,
) -> WorktreeAction {
    WorktreeAction::Copy {
        paths: owned(paths),
        directory: directory.to_string(),
        on_conflict,
    }
}

pub(super) fn rename_action(path: &str, new_name: &str) -> WorktreeAction {
    WorktreeAction::Rename {
        path: path.to_string(),
        new_name: new_name.to_string(),
    }
}

fn owned(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|path| path.to_string()).collect()
}
