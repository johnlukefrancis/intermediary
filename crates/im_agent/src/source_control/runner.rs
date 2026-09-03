// Path: crates/im_agent/src/source_control/runner.rs
// Description: spawn_blocking bridge and Git failure to AgentError mapping for source control

//! Every Git process for source control starts here: the repo root is checked
//! before spawning (a moved repo must not look like a missing binary), the
//! installed Git version is probed once per process, arguments are built from
//! `common_git_args()`, and the bounded runner executes on `spawn_blocking`
//! (ADR-009). Reads carry the request's cancel token and may be killed at once;
//! mutations carry no token and stop gracefully only on timeout, because an
//! abrupt kill can leave `.git/index.lock` behind.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use im_bundle::git::{
    capture_repo_prefix, common_git_args, run_git_with_input, BundleCancelToken, GitCommandOutput,
    KillPolicy,
};

use crate::error::AgentError;

use super::git_version;
use super::runner_failure::{map_failure, map_runner_error};

pub(super) const READ_TIMEOUT: Duration = Duration::from_secs(20);
pub(super) const INDEX_TIMEOUT: Duration = Duration::from_secs(60);
pub(super) const COMMIT_TIMEOUT: Duration = Duration::from_secs(120);
pub(super) const REMOTE_TIMEOUT: Duration = Duration::from_secs(180);
pub(super) const STATUS_LIMIT: usize = 8 * 1024 * 1024;
pub(super) const DIFF_LIMIT: usize = 2 * 1024 * 1024;
pub(super) const ACTION_LIMIT: usize = 1024 * 1024;

/// Bounds for the small probes that support a command rather than serving a
/// request: the version check and the leftover-lock lookup.
pub(super) const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const PROBE_LIMIT: usize = 4096;

pub(super) fn git_executable() -> PathBuf {
    PathBuf::from("git")
}

/// One Git invocation: subcommand arguments (appended after
/// `common_git_args()`), optional stdin, accepted non-zero exit codes, and the
/// output/time bounds. Defaults suit a small read.
pub(super) struct GitCall {
    args: Vec<OsString>,
    stdin: Option<Vec<u8>>,
    accepted_nonzero_codes: Vec<i32>,
    stdout_limit: usize,
    timeout: Duration,
}

impl GitCall {
    pub(super) fn new<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        Self {
            args: args.into_iter().map(Into::into).collect(),
            stdin: None,
            accepted_nonzero_codes: Vec::new(),
            stdout_limit: ACTION_LIMIT,
            timeout: READ_TIMEOUT,
        }
    }

    pub(super) fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub(super) fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub(super) fn stdin(mut self, bytes: Vec<u8>) -> Self {
        self.stdin = Some(bytes);
        self
    }

    pub(super) fn accept_exit_codes(mut self, codes: &[i32]) -> Self {
        self.accepted_nonzero_codes = codes.to_vec();
        self
    }

    pub(super) fn stdout_limit(mut self, limit: usize) -> Self {
        self.stdout_limit = limit;
        self
    }

    pub(super) fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    Read,
    Mutation,
}

/// Runs a read-only command with the request's cancel token; cancellation
/// kills the child immediately.
pub(super) async fn run_read(
    repo_root: &Path,
    call: GitCall,
    cancel_token: Option<BundleCancelToken>,
) -> Result<GitCommandOutput, AgentError> {
    let repo_root = repo_root.to_path_buf();
    spawn(move || run_blocking(&repo_root, call, cancel_token.as_ref(), Mode::Read)).await
}

/// Runs a mutation: no cancel token, graceful stop only on timeout.
pub(super) async fn run_mutation(
    repo_root: &Path,
    call: GitCall,
) -> Result<GitCommandOutput, AgentError> {
    let repo_root = repo_root.to_path_buf();
    spawn(move || run_blocking(&repo_root, call, None, Mode::Mutation)).await
}

/// Where a configured root sits in its repository: the `--show-prefix` bytes
/// with a trailing slash (empty at the top level) and the physical Git
/// directory its index lives in.
pub(super) struct RepoLocation {
    pub prefix: Vec<u8>,
    pub git_dir: PathBuf,
}

/// One `git rev-parse --show-prefix --absolute-git-dir`, through the same
/// pre-checks and failure mapping as reads. Both answers come from this single
/// process, so a status read never spawns Git again to learn which index it is
/// looking at.
pub(super) async fn capture_location(
    repo_root: &Path,
    cancel_token: Option<BundleCancelToken>,
) -> Result<RepoLocation, AgentError> {
    let repo_root = repo_root.to_path_buf();
    spawn(move || {
        prepare(&repo_root)?;
        let captured =
            capture_repo_prefix(&git_executable(), &repo_root, READ_TIMEOUT, cancel_token.as_ref())
                .map_err(map_runner_error)?
                .map_err(|failure| map_failure(Mode::Read, &repo_root, READ_TIMEOUT, failure))?;
        if captured.truncated {
            return Err(AgentError::new(
                "GIT_COMMAND_FAILED",
                "Git repository prefix exceeded its output bound",
            ));
        }
        let git_dir = captured.git_dir.ok_or_else(|| {
            AgentError::new(
                "GIT_NOT_REPOSITORY",
                format!("Git reported no git directory for {}", repo_root.display()),
            )
        })?;
        Ok(RepoLocation {
            prefix: captured.prefix,
            git_dir,
        })
    })
    .await
}

async fn spawn<T, F>(work: F) -> Result<T, AgentError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, AgentError> + Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .unwrap_or_else(|error| Err(AgentError::internal(format!("Git task failed: {error}"))))
}

fn prepare(repo_root: &Path) -> Result<(), AgentError> {
    if !repo_root.is_dir() {
        return Err(AgentError::new(
            "INVALID_REPO",
            format!("Repo root no longer exists: {}", repo_root.display()),
        ));
    }
    git_version::ensure_supported(repo_root)
}

fn run_blocking(
    repo_root: &Path,
    call: GitCall,
    cancel_token: Option<&BundleCancelToken>,
    mode: Mode,
) -> Result<GitCommandOutput, AgentError> {
    prepare(repo_root)?;
    let mut args = common_git_args();
    args.extend(call.args);
    let kill_policy = match mode {
        Mode::Read => KillPolicy::Immediate,
        Mode::Mutation => KillPolicy::Graceful,
    };
    run_git_with_input(
        &git_executable(),
        repo_root,
        &args,
        call.stdin,
        &call.accepted_nonzero_codes,
        call.stdout_limit,
        call.timeout,
        cancel_token,
        kill_policy,
    )
    .map_err(map_runner_error)?
    .map_err(|failure| map_failure(mode, repo_root, call.timeout, failure))
}
