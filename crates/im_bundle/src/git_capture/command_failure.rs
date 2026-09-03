// Path: crates/im_bundle/src/git_capture/command_failure.rs
// Description: Why a Git command produced no usable output, and the bounded streams it failed with

use super::command_child::Collected;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitCommandFailureKind {
    MissingExecutable,
    TimedOut,
    SpawnFailed,
    InputWriteFailed,
    OutputReadFailed,
    NotGitRepository,
    NonZeroExit,
    /// No owner could be created for the tree this command would start.
    /// `spawned` is false when the refusal came before any spawn and true
    /// when the child existed for the instant between spawn and attach and
    /// was killed at once — the caller may prove nothing about the first
    /// instructions Git ran in that instant.
    NoProcessTreeOwner { spawned: bool },
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
    pub(super) fn bare(kind: GitCommandFailureKind) -> Self {
        Self {
            kind,
            exit_code: None,
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }

    pub(super) fn from_streams(kind: GitCommandFailureKind, output: Collected) -> Self {
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
