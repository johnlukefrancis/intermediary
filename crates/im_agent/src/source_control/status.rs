// Path: crates/im_agent/src/source_control/status.rs
// Description: Capture `git status --porcelain=v2` for one repo root and project it onto the wire shape

use std::path::Path;

use im_bundle::git::BundleCancelToken;

use crate::error::AgentError;
use crate::protocol::SourceControlStatus;

use super::runner::{self, GitCall, READ_TIMEOUT, STATUS_LIMIT};
use super::status_project::project_status;

/// Step 0 captures the repo prefix (`rev-parse --show-prefix`), then the
/// whole-repository status is read with a bounded budget (a truncated capture
/// is projected best-effort and flagged), then Git decides `committable`.
pub(super) async fn capture_status(
    repo_root: &Path,
    cancel_token: Option<BundleCancelToken>,
) -> Result<SourceControlStatus, AgentError> {
    let prefix = runner::capture_prefix(repo_root, cancel_token.clone()).await?;
    let call = GitCall::new([
        "-c",
        "status.relativePaths=false",
        "status",
        "--porcelain=v2",
        "-z",
        "--branch",
        "--untracked-files=all",
        "--ignore-submodules=none",
    ])
    .stdout_limit(STATUS_LIMIT)
    .timeout(READ_TIMEOUT);
    let output = runner::run_read(repo_root, call, cancel_token.clone()).await?;
    let committable = capture_committable(repo_root, cancel_token).await?;
    project_status(&prefix, output, committable)
}

/// Whether `git commit` would accept the index: it differs from HEAD (the
/// empty tree on an unborn branch), or a merge is being concluded, which Git
/// records even when the resolved tree equals HEAD. The projected `index`
/// list cannot tell either case from "nothing staged", so Git is asked with
/// two bounded reads; each accepts exit 1 as its "no" answer.
async fn capture_committable(
    repo_root: &Path,
    cancel_token: Option<BundleCancelToken>,
) -> Result<bool, AgentError> {
    let cached =
        GitCall::new(["diff", "--cached", "--quiet", "--no-ext-diff"]).accept_exit_codes(&[1]);
    let index = runner::run_read(repo_root, cached, cancel_token.clone()).await?;
    if index.exit_code == 1 {
        return Ok(true);
    }
    let probe = GitCall::new(["rev-parse", "-q", "--verify", "MERGE_HEAD"]).accept_exit_codes(&[1]);
    let merge_head = runner::run_read(repo_root, probe, cancel_token).await?;
    Ok(merge_head.exit_code == 0)
}
