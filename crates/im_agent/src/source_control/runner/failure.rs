// Path: crates/im_agent/src/source_control/runner/failure.rs
// Description: Maps a Git command failure onto an AgentError and, for mutations, its proven effect

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use im_bundle::error::BundleError;
use im_bundle::git::{
    bytes_to_path, common_git_args, run_git, trim_line_ending, GitCommandFailure,
    GitCommandFailureKind,
};

use crate::error::{AgentError, MutationEffect};

use super::{git_executable, Mode, PROBE_LIMIT, PROBE_TIMEOUT};

pub(super) fn map_runner_error(error: BundleError) -> AgentError {
    match error {
        BundleError::Cancelled => AgentError::new("CANCELLED", "Source control request cancelled"),
        other => AgentError::internal(format!("Git runner failed: {other}")),
    }
}

/// Maps a read-side probe failure (used by the version probe before any
/// command-specific context exists).
pub(super) fn map_probe_failure(repo_root: &Path, failure: GitCommandFailure) -> AgentError {
    map_failure(Mode::Read, repo_root, PROBE_TIMEOUT, failure)
}

/// Maps one Git failure, and for a mutation records what it proves about the
/// repository. A process that never started changed nothing; a process stopped
/// on its timeout, or whose pipes failed, may have crossed its effect boundary
/// already. A non-zero exit is left unclassified: only the caller knows whether
/// its command is atomic, and `git pull` is not `git add`.
pub(super) fn map_failure(
    mode: Mode,
    repo_root: &Path,
    timeout: Duration,
    failure: GitCommandFailure,
) -> AgentError {
    let effect = match mode {
        Mode::Mutation => effect_of(&failure.kind),
        Mode::Read => None,
    };
    let error = map_failure_code(mode, repo_root, timeout, failure);
    match effect {
        Some(effect) => error.with_effect(effect),
        None => error,
    }
}

/// What a failure proves about the repository. A process that never started
/// changed nothing — including one refused for want of a process tree owner,
/// which is decided before the spawn, or else kills the child in the same
/// breath, before Git can take a lock. A process stopped on its timeout, or
/// one whose pipes failed, may already have crossed its effect boundary. A
/// non-zero exit is left unclassified: only the caller knows whether its
/// command is atomic, and `git pull` is not `git add`.
fn effect_of(kind: &GitCommandFailureKind) -> Option<MutationEffect> {
    match kind {
        GitCommandFailureKind::MissingExecutable
        | GitCommandFailureKind::NotGitRepository
        | GitCommandFailureKind::SpawnFailed
        | GitCommandFailureKind::NoProcessTreeOwner { spawned: false } => {
            Some(MutationEffect::NotApplied)
        }
        GitCommandFailureKind::NoProcessTreeOwner { spawned: true }
        | GitCommandFailureKind::TimedOut
        | GitCommandFailureKind::InputWriteFailed
        | GitCommandFailureKind::OutputReadFailed => Some(MutationEffect::Unknown),
        GitCommandFailureKind::NonZeroExit => None,
    }
}

fn map_failure_code(
    mode: Mode,
    repo_root: &Path,
    timeout: Duration,
    failure: GitCommandFailure,
) -> AgentError {
    let seconds = timeout.as_secs();
    match failure.kind {
        GitCommandFailureKind::MissingExecutable => {
            AgentError::new("GIT_UNAVAILABLE", "Git executable not found on PATH")
        }
        GitCommandFailureKind::NotGitRepository => AgentError::new(
            "GIT_NOT_REPOSITORY",
            non_empty(failure.message(), "Not a Git repository"),
        ),
        GitCommandFailureKind::TimedOut => match (mode, leftover_index_lock(repo_root)) {
            (Mode::Mutation, Some(lock)) => AgentError::new(
                "GIT_ABORTED",
                format!(
                    "Git was stopped after {seconds}s and left {} behind; remove it once no Git process is running",
                    lock.display()
                ),
            ),
            _ => AgentError::new("GIT_TIMEOUT", format!("Git timed out after {seconds}s")),
        },
        GitCommandFailureKind::NonZeroExit => AgentError::new(
            "GIT_COMMAND_FAILED",
            non_empty(
                failure.message(),
                &format!(
                    "Git exited with status {}",
                    failure.exit_code.map_or("unknown".to_string(), |c| c.to_string())
                ),
            ),
        ),
        GitCommandFailureKind::SpawnFailed => AgentError::new(
            "GIT_COMMAND_FAILED",
            non_empty(failure.message(), "Git could not be started"),
        ),
        GitCommandFailureKind::InputWriteFailed => AgentError::new(
            "GIT_COMMAND_FAILED",
            non_empty(failure.message(), "Git closed its input before reading it"),
        ),
        GitCommandFailureKind::OutputReadFailed => AgentError::new(
            "GIT_COMMAND_FAILED",
            non_empty(failure.message(), "Git output could not be read"),
        ),
        GitCommandFailureKind::NoProcessTreeOwner { spawned: false } => AgentError::new(
            "GIT_COMMAND_FAILED",
            non_empty(
                failure.message(),
                "Git was not started: its process tree could not be owned",
            ),
        ),
        GitCommandFailureKind::NoProcessTreeOwner { spawned: true } => AgentError::new(
            "GIT_COMMAND_FAILED",
            non_empty(
                failure.message(),
                "Git was stopped the instant it started: its process tree could not be owned",
            ),
        ),
    }
}

fn non_empty(message: String, fallback: &str) -> String {
    if message.is_empty() {
        fallback.to_string()
    } else {
        message
    }
}

/// Where a stopped mutation would have left its index lock, when it did.
fn leftover_index_lock(repo_root: &Path) -> Option<PathBuf> {
    let mut args = common_git_args();
    args.extend(["rev-parse", "--git-path", "index.lock"].map(OsString::from));
    let derived = run_git(&git_executable(), repo_root, &args, PROBE_LIMIT, PROBE_TIMEOUT, None)
        .ok()
        .and_then(Result::ok)
        .and_then(|output| bytes_to_path(&trim_line_ending(output.stdout)))
        .map(|path| if path.is_absolute() { path } else { repo_root.join(path) })
        .unwrap_or_else(|| repo_root.join(".git").join("index.lock"));
    derived.exists().then_some(derived)
}

#[cfg(test)]
mod tests {
    use super::{map_failure, Mode};
    use im_bundle::git::{GitCommandFailure, GitCommandFailureKind};
    use std::path::Path;
    use std::time::Duration;

    /// The runner refuses the command before the spawn, or kills the child in
    /// the same breath, so the repository is untouched — and the caller is told
    /// so rather than left to guess.
    #[test]
    fn a_mutation_refused_for_want_of_a_tree_owner_never_applied() {
        let failure = GitCommandFailure {
            kind: GitCommandFailureKind::NoProcessTreeOwner { spawned: false },
            exit_code: None,
            stdout: Vec::new(),
            stderr: b"Git process tree owner unavailable: Access is denied.".to_vec(),
        };

        let error = map_failure(
            Mode::Mutation,
            Path::new("/repo"),
            Duration::from_secs(30),
            failure,
        );

        assert_eq!(error.code(), "GIT_COMMAND_FAILED");
        assert_eq!(error.effect(), Some("notApplied"));
        assert!(error.message().contains("process tree owner"));
    }

    /// Once the child existed, even for the instant before its attach failed,
    /// nothing proves what it ran; the honest effect is unknown.
    #[test]
    fn a_mutation_whose_child_could_not_be_attached_is_unknown() {
        let failure = GitCommandFailure {
            kind: GitCommandFailureKind::NoProcessTreeOwner { spawned: true },
            exit_code: None,
            stdout: Vec::new(),
            stderr: b"Git process tree owner unavailable: Access is denied.".to_vec(),
        };

        let error = map_failure(
            Mode::Mutation,
            Path::new("/repo"),
            Duration::from_secs(30),
            failure,
        );

        assert_eq!(error.effect(), Some("unknown"));
        assert!(error.message().contains("process tree owner"));
    }
}
