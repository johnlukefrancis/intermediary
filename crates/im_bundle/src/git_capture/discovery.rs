// Path: crates/im_bundle/src/git_capture/discovery.rs
// Description: Git discovery failure classification and raw prefix normalization

use super::command::{GitCommandFailure, GitCommandFailureKind};
use super::{GitCaptureIssue, GIT_STATUS_NAME};

pub(crate) fn initial_issue(failure: GitCommandFailure) -> GitCaptureIssue {
    let (kind, detail) = match failure.kind {
        GitCommandFailureKind::MissingExecutable => (
            "gitUnavailable",
            "The Git executable is not available on this bundle-building host.",
        ),
        GitCommandFailureKind::TimedOut => (
            "commandTimeout",
            "The bounded Git discovery/status command timed out.",
        ),
        GitCommandFailureKind::NotGitRepository => (
            "notGitRepository",
            "Git did not recognize the configured root as a usable working tree.",
        ),
        GitCommandFailureKind::NonZeroExit => (
            "commandFailure",
            "The Git discovery/status command returned a non-zero status.",
        ),
        GitCommandFailureKind::SpawnFailed
        | GitCommandFailureKind::InputWriteFailed
        | GitCommandFailureKind::OutputReadFailed
        | GitCommandFailureKind::NoProcessTreeOwner { .. } => (
            "commandFailure",
            "The Git discovery/status command could not be executed or read.",
        ),
    };
    GitCaptureIssue::new(kind, Some(GIT_STATUS_NAME), detail)
}

pub fn trim_line_ending(mut bytes: Vec<u8>) -> Vec<u8> {
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
    }
    if bytes.last() == Some(&b'\r') {
        bytes.pop();
    }
    bytes
}
