// Path: crates/im_agent/src/source_control/actions/remote.rs
// Description: Push and pull for one repo root, including upstream selection

use std::path::Path;

use crate::error::{AgentError, MutationEffect};

use crate::source_control::locks::SourceControlLocks;
use crate::source_control::runner::{self, GitCall, REMOTE_TIMEOUT};
use crate::source_control::status;

/// With an upstream, plain `push`; with none and exactly one remote, publish
/// the current branch there; anything else needs the user to decide.
pub(super) async fn push(repo_root: &Path, locks: &SourceControlLocks) -> Result<(), AgentError> {
    let status = status::capture_status(repo_root, None, locks)
        .await
        .map_err(|error| error.with_effect(MutationEffect::NotApplied))?
        .status;
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
                )
                .with_effect(MutationEffect::NotApplied))
            }
        }
    };
    run_remote(repo_root, call).await
}

pub(super) async fn pull(repo_root: &Path) -> Result<(), AgentError> {
    run_remote(repo_root, GitCall::new(["pull", "--ff-only"])).await
}

/// A failed remote command is never proof that nothing changed: a fetch can
/// have updated refs, and a pull can have left a merge in progress, so the
/// outcome stays unknown unless a site proved otherwise.
async fn run_remote(repo_root: &Path, call: GitCall) -> Result<(), AgentError> {
    runner::run_mutation(repo_root, call.timeout(REMOTE_TIMEOUT))
        .await
        .map(drop)
        .map_err(|error| error.with_default_effect(MutationEffect::Unknown))
}

async fn list_remotes(repo_root: &Path) -> Result<Vec<String>, AgentError> {
    let output = runner::run_read(repo_root, GitCall::new(["remote"]), None)
        .await
        .map_err(|error| error.with_effect(MutationEffect::NotApplied))?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(String::from)
        .collect())
}
