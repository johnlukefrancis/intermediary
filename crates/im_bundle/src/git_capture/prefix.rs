// Path: crates/im_bundle/src/git_capture/prefix.rs
// Description: Shared bounded capture of the Git repository prefix and absolute git dir for a configured root

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::cancel::BundleCancelToken;
use crate::error::Result;

use super::command::{run_git, GitCommandFailure};
use super::diff::common_git_args;
use super::discovery::trim_line_ending;
use super::path::bytes_to_path;

const PREFIX_LIMIT: usize = 1024 * 1024;

/// The two facts one `rev-parse` settles about a configured root. They are
/// captured together because a status read must not pay for a second Git
/// process to learn either one.
#[derive(Debug, Clone)]
pub struct RepoPrefixCapture {
    /// The path of `repo_root` relative to the Git top level, as raw bytes
    /// with a trailing slash (empty when the root is the top level itself).
    /// Porcelain paths are top-level-relative, so every consumer strips this.
    pub prefix: Vec<u8>,
    /// `git rev-parse --absolute-git-dir`: the physical index this root works
    /// on, shared by a repository root and any configured subdirectory below
    /// it and distinct for a linked worktree. `None` only where the platform
    /// cannot represent Git's bytes as a path, which never happens on Unix.
    pub git_dir: Option<PathBuf>,
    pub truncated: bool,
}

pub fn capture_repo_prefix(
    executable: &Path,
    repo_root: &Path,
    timeout: Duration,
    cancel_token: Option<&BundleCancelToken>,
) -> Result<std::result::Result<RepoPrefixCapture, GitCommandFailure>> {
    let mut args = common_git_args();
    args.extend([
        OsString::from("rev-parse"),
        OsString::from("--show-prefix"),
        OsString::from("--absolute-git-dir"),
    ]);
    let output = run_git(executable, repo_root, &args, PREFIX_LIMIT, timeout, cancel_token)?;
    Ok(output.map(|output| {
        let (prefix, git_dir) = split_answers(output.stdout);
        RepoPrefixCapture {
            prefix,
            git_dir,
            truncated: output.stdout_truncated,
        }
    }))
}

/// `rev-parse` answers in argument order, one line each. The prefix line is
/// empty at the top level, so the split happens on the first newline before any
/// trimming; trimming first would swallow the empty answer and leave the git
/// dir standing in for the prefix.
fn split_answers(stdout: Vec<u8>) -> (Vec<u8>, Option<PathBuf>) {
    let Some(newline) = stdout.iter().position(|byte| *byte == b'\n') else {
        return (trim_line_ending(stdout), None);
    };
    let git_dir = trim_line_ending(stdout[newline + 1..].to_vec());
    let mut prefix = stdout;
    prefix.truncate(newline);
    let git_dir = if git_dir.is_empty() {
        None
    } else {
        bytes_to_path(&git_dir)
    };
    (trim_line_ending(prefix), git_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    const TIMEOUT: Duration = Duration::from_secs(20);

    fn git(dir: &Path, args: &[&str]) {
        assert!(Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .expect("run git")
            .success());
    }

    fn capture(repo_root: &Path) -> RepoPrefixCapture {
        capture_repo_prefix(Path::new("git"), repo_root, TIMEOUT, None)
            .expect("runner result")
            .expect("git answered")
    }

    /// A configured subdirectory names the same physical index as the root
    /// above it; only its prefix differs.
    #[test]
    fn a_root_and_a_subdirectory_report_one_git_dir_and_different_prefixes() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("repo");
        std::fs::create_dir_all(root.join("sub")).expect("repo dirs");
        git(&root, &["init", "-q", "-b", "main"]);
        git(&root, &["config", "user.email", "prefix@example.test"]);
        git(&root, &["config", "user.name", "Prefix Test"]);
        std::fs::write(root.join("sub/file.txt"), "content\n").expect("file");
        git(&root, &["add", "."]);
        git(&root, &["commit", "-qm", "baseline"]);

        let top = capture(&root);
        let below = capture(&root.join("sub"));
        assert!(top.prefix.is_empty());
        assert_eq!(below.prefix, b"sub/".to_vec());
        assert!(!top.truncated && !below.truncated);
        let git_dir = top.git_dir.expect("top-level git dir");
        assert_eq!(git_dir.file_name().expect("dir name"), ".git");
        assert_eq!(below.git_dir.expect("subdirectory git dir"), git_dir);
    }

    /// A linked worktree has its own index and must report its own git dir,
    /// never the primary worktree's.
    #[test]
    fn a_linked_worktree_reports_its_own_git_dir() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().join("repo");
        std::fs::create_dir_all(&root).expect("repo dir");
        git(&root, &["init", "-q", "-b", "main"]);
        git(&root, &["config", "user.email", "prefix@example.test"]);
        git(&root, &["config", "user.name", "Prefix Test"]);
        std::fs::write(root.join("file.txt"), "content\n").expect("file");
        git(&root, &["add", "."]);
        git(&root, &["commit", "-qm", "baseline"]);
        let linked = temp.path().join("linked");
        git(
            &root,
            &[
                "worktree",
                "add",
                "-q",
                linked.to_str().expect("utf8"),
                "-b",
                "linked",
            ],
        );

        let primary = capture(&root).git_dir.expect("primary git dir");
        let secondary = capture(&linked).git_dir.expect("linked git dir");
        assert_ne!(secondary, primary);
        assert!(capture(&linked).prefix.is_empty());
    }
}
