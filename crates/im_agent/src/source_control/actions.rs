// Path: crates/im_agent/src/source_control/actions.rs
// Description: Stage, unstage, commit, push, and pull for one repo root; every mutation returns a fresh status

use std::path::Path;

use im_bundle::git::trim_line_ending;

use crate::error::AgentError;
use crate::protocol::{SourceControlActionKind, SourceControlActionPayload, SourceControlScope};

use super::actions_discard::discard;
use super::paths::{normalize_paths, nul_joined, PATHSPEC_FROM_STDIN};
use super::runner::{self, GitCall, COMMIT_TIMEOUT, INDEX_TIMEOUT, REMOTE_TIMEOUT};
use super::{status, SourceControlActionOutcome};

/// Runs the mutation, then its follow-up reads: `rev-parse HEAD` for a commit
/// and the fresh status for every kind. Once the mutation has landed, a failing
/// follow-up read never surfaces as a `GIT_*` error; the UI treats the
/// non-`GIT_*` code as unknown-outcome and reconciles by refetching.
pub(super) async fn run_action(
    repo_root: &Path,
    action: SourceControlActionPayload,
) -> Result<SourceControlActionOutcome, AgentError> {
    let kind = action.kind();
    let committed = match action {
        SourceControlActionPayload::Stage { scope } => {
            stage(repo_root, scope).await?;
            false
        }
        SourceControlActionPayload::Unstage { scope } => {
            unstage(repo_root, scope).await?;
            false
        }
        SourceControlActionPayload::Discard { paths } => {
            discard(repo_root, &paths).await?;
            false
        }
        SourceControlActionPayload::Commit { message } => {
            commit(repo_root, &message).await?;
            true
        }
        SourceControlActionPayload::Push => {
            push(repo_root).await?;
            false
        }
        SourceControlActionPayload::Pull => {
            pull(repo_root).await?;
            false
        }
    };
    let commit_sha = if committed {
        let sha = head_sha(repo_root)
            .await
            .map_err(|error| applied_but_unread(kind, None, error))?;
        Some(sha)
    } else {
        None
    };
    let status = status::capture_status(repo_root, None)
        .await
        .map_err(|error| applied_but_unread(kind, commit_sha.clone(), error))?;
    Ok(SourceControlActionOutcome { status, commit_sha })
}

fn applied_but_unread(
    kind: SourceControlActionKind,
    commit_sha: Option<String>,
    inner: AgentError,
) -> AgentError {
    let name = format!("{kind:?}").to_ascii_lowercase();
    AgentError::new(
        "ACTION_APPLIED_STATUS_UNAVAILABLE",
        format!(
            "{name} completed but the follow-up status read failed: {}",
            inner.message()
        ),
    )
    .with_details(serde_json::json!({ "kind": kind, "commitSha": commit_sha }))
}

/// `All` stays inside the configured root (`-- .`); explicit paths travel
/// NUL-separated on stdin. An empty list is a no-op: zero pathspecs would mean
/// the whole repository to Git.
async fn stage(repo_root: &Path, scope: SourceControlScope) -> Result<(), AgentError> {
    let call = match scope {
        SourceControlScope::All => GitCall::new(["add", "-A", "--", "."]),
        SourceControlScope::Paths { paths } => {
            let Some(input) = pathspec_input(&paths)? else {
                return Ok(());
            };
            GitCall::new(["add", "-A"])
                .args(PATHSPEC_FROM_STDIN)
                .stdin(input)
        }
    };
    runner::run_mutation(repo_root, call.timeout(INDEX_TIMEOUT))
        .await
        .map(drop)
}

/// `reset` without a commit resolves to the empty tree on an unborn branch,
/// so unstaging works before the first commit.
async fn unstage(repo_root: &Path, scope: SourceControlScope) -> Result<(), AgentError> {
    let call = match scope {
        SourceControlScope::All => GitCall::new(["reset", "-q", "--", "."]),
        SourceControlScope::Paths { paths } => {
            let Some(input) = pathspec_input(&paths)? else {
                return Ok(());
            };
            GitCall::new(["reset", "-q"])
                .args(PATHSPEC_FROM_STDIN)
                .stdin(input)
        }
    };
    runner::run_mutation(repo_root, call.timeout(INDEX_TIMEOUT))
        .await
        .map(drop)
}

fn pathspec_input(paths: &[String]) -> Result<Option<Vec<u8>>, AgentError> {
    if paths.is_empty() {
        return Ok(None);
    }
    Ok(Some(nul_joined(&normalize_paths(paths)?)))
}

/// Commits the whole index (a partial `commit -- <paths>` is never issued).
/// Refused up front unless the status oracle says Git would accept a commit,
/// so the UI sees a typed code instead of Git's stdout explanation.
async fn commit(repo_root: &Path, message: &str) -> Result<(), AgentError> {
    if message.trim().is_empty() {
        return Err(AgentError::new(
            "INVALID_COMMIT_MESSAGE",
            "Commit message must not be blank",
        ));
    }
    let status = status::capture_status(repo_root, None).await?;
    if !status.committable {
        return Err(AgentError::new(
            "GIT_NOTHING_TO_COMMIT",
            "Nothing is staged to commit",
        ));
    }
    let call = GitCall::new(["commit", "-q", "--cleanup=whitespace", "-F", "-"])
        .stdin(message.as_bytes().to_vec())
        .timeout(COMMIT_TIMEOUT);
    runner::run_mutation(repo_root, call).await.map(drop)
}

async fn head_sha(repo_root: &Path) -> Result<String, AgentError> {
    let output = runner::run_read(repo_root, GitCall::new(["rev-parse", "HEAD"]), None).await?;
    let sha = String::from_utf8_lossy(&trim_line_ending(output.stdout))
        .trim()
        .to_string();
    if sha.is_empty() {
        return Err(AgentError::new(
            "GIT_COMMAND_FAILED",
            "Git reported no HEAD after the commit",
        ));
    }
    Ok(sha)
}

/// With an upstream, plain `push`; with none and exactly one remote, publish
/// the current branch there; anything else needs the user to decide.
async fn push(repo_root: &Path) -> Result<(), AgentError> {
    let status = status::capture_status(repo_root, None).await?;
    let call = if status.upstream.is_some() {
        GitCall::new(["push"])
    } else {
        let remotes = list_remotes(repo_root).await?;
        match remotes.as_slice() {
            [remote] => GitCall::new(["push", "-u"]).arg(remote.as_str()).arg("HEAD"),
            _ => {
                return Err(AgentError::new(
                    "GIT_COMMAND_FAILED",
                    "No upstream; configure one remote or set an upstream",
                ))
            }
        }
    };
    runner::run_mutation(repo_root, call.timeout(REMOTE_TIMEOUT))
        .await
        .map(drop)
}

async fn list_remotes(repo_root: &Path) -> Result<Vec<String>, AgentError> {
    let output = runner::run_read(repo_root, GitCall::new(["remote"]), None).await?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(String::from)
        .collect())
}

async fn pull(repo_root: &Path) -> Result<(), AgentError> {
    let call = GitCall::new(["pull", "--ff-only"]).timeout(REMOTE_TIMEOUT);
    runner::run_mutation(repo_root, call).await.map(drop)
}
