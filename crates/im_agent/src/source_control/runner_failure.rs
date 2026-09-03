// Path: crates/im_agent/src/source_control/runner_failure.rs
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

use super::runner::{git_executable, Mode, PROBE_LIMIT, PROBE_TIMEOUT};

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
/// changed nothing. A process stopped on its timeout, or one whose pipes
/// failed, may already have crossed its effect boundary. A non-zero exit is
/// left unclassified: only the caller knows whether its command is atomic, and
/// `git pull` is not `git add`.
fn effect_of(kind: &GitCommandFailureKind) -> Option<MutationEffect> {
    match kind {
        GitCommandFailureKind::MissingExecutable
        | GitCommandFailureKind::NotGitRepository
        | GitCommandFailureKind::SpawnFailed => Some(MutationEffect::NotApplied),
        GitCommandFailureKind::TimedOut
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
