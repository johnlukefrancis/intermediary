// Path: crates/im_bundle/src/git_capture/diff_issue.rs
// Description: Artifact-specific issue classification for selected Git diff capture

use super::command::{GitCommandFailure, GitCommandFailureKind};
use super::pathspec_batches::PathspecBatchError;
use super::{
    GitCaptureIssue, GIT_DIFF_NAME, GIT_INDEX_DIFF_NAME, GIT_STATUS_NAME, GIT_WORKTREE_DIFF_NAME,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DiffOutput {
    Patch,
    IndexPatch,
    WorktreePatch,
    Stat,
    NameStatus,
}

impl DiffOutput {
    pub(super) fn artifact(self) -> &'static str {
        match self {
            Self::Patch => GIT_DIFF_NAME,
            Self::IndexPatch => GIT_INDEX_DIFF_NAME,
            Self::WorktreePatch => GIT_WORKTREE_DIFF_NAME,
            Self::Stat | Self::NameStatus => GIT_STATUS_NAME,
        }
    }
}

pub(super) fn command_issue(
    output_kind: DiffOutput,
    failure: GitCommandFailure,
) -> GitCaptureIssue {
    let (kind, detail) = match failure.kind {
        GitCommandFailureKind::MissingExecutable => (
            "gitUnavailable",
            "The Git executable became unavailable during evidence capture.",
        ),
        GitCommandFailureKind::TimedOut => (
            "commandTimeout",
            "A bounded Git evidence command timed out.",
        ),
        GitCommandFailureKind::SpawnFailed
        | GitCommandFailureKind::InputWriteFailed
        | GitCommandFailureKind::OutputReadFailed
        | GitCommandFailureKind::NoProcessTreeOwner { .. } => (
            "commandFailure",
            "A Git evidence command could not be executed or read.",
        ),
        GitCommandFailureKind::NotGitRepository | GitCommandFailureKind::NonZeroExit => (
            "commandFailure",
            "A Git evidence command returned a non-zero status.",
        ),
    };
    GitCaptureIssue::new(kind, Some(output_kind.artifact()), detail)
}

pub(super) fn pathspec_issue(
    output_kind: DiffOutput,
    error: PathspecBatchError,
) -> GitCaptureIssue {
    match error {
        PathspecBatchError::UnsupportedEncoding => GitCaptureIssue::new(
            "unsupportedPathEncoding",
            Some(output_kind.artifact()),
            "At least one selected Git path could not be passed to Git on this host.",
        ),
        PathspecBatchError::AtomicGroupTooLarge => GitCaptureIssue::new(
            "pathLimit",
            Some(output_kind.artifact()),
            "A selected path or rename pair exceeded the host-safe Git argument batch budget.",
        ),
    }
}

pub(super) fn limit_issue(output_kind: DiffOutput) -> GitCaptureIssue {
    GitCaptureIssue::new(
        "outputTruncated",
        Some(output_kind.artifact()),
        "The artifact output safety bound was exhausted.",
    )
}
