// Path: crates/im_agent/src/source_control/diff.rs
// Description: Bounded per-file unified diff capture for one repo root (index, worktree, or untracked)

use std::path::Path;

use im_bundle::git::BundleCancelToken;

use crate::error::AgentError;
use crate::protocol::SourceControlArea;

use super::paths::normalize_path;
use super::runner::{self, GitCall, DIFF_LIMIT, READ_TIMEOUT};
use super::SourceControlDiff;

const DIFF_FLAGS: [&str; 5] = [
    "--no-ext-diff",
    "--no-textconv",
    "--no-color",
    "--unified=3",
    "--find-renames",
];

/// Tracked paths diff through the index (`--cached` for the index area); an
/// untracked worktree path is shown as all-added against `/dev/null`, which
/// Git special-cases on every platform. `core.quotePath=false` (overriding the
/// shared profile's `true`) keeps non-ASCII names readable in the patch
/// headers; the patch is displayed, never parsed as paths.
pub(super) async fn capture_diff(
    repo_root: &Path,
    path: &str,
    original_path: Option<&str>,
    area: SourceControlArea,
    cancel_token: Option<BundleCancelToken>,
) -> Result<SourceControlDiff, AgentError> {
    let path = normalize_path(path)?;
    let original = original_path.map(normalize_path).transpose()?;
    let tracked = match area {
        SourceControlArea::Index => true,
        SourceControlArea::Worktree => is_tracked(repo_root, &path, cancel_token.clone()).await?,
    };
    let call = if tracked {
        tracked_diff(&path, original.as_deref(), area)
    } else {
        untracked_diff(&path)
    };
    let output = runner::run_read(repo_root, call, cancel_token).await?;
    let patch = String::from_utf8_lossy(&output.stdout).into_owned();
    let binary = patch
        .lines()
        .any(|line| line.starts_with("Binary files ") && line.ends_with(" differ"));
    Ok(SourceControlDiff {
        patch,
        truncated: output.stdout_truncated,
        binary,
    })
}

/// `ls-files` prints nothing for a path absent from the index.
async fn is_tracked(
    repo_root: &Path,
    path: &str,
    cancel_token: Option<BundleCancelToken>,
) -> Result<bool, AgentError> {
    let call = GitCall::new(["ls-files", "-z", "--"]).arg(path);
    let output = runner::run_read(repo_root, call, cancel_token).await?;
    Ok(!output.stdout.is_empty())
}

const UNQUOTED_PATHS: [&str; 2] = ["-c", "core.quotePath=false"];

fn tracked_diff(path: &str, original: Option<&str>, area: SourceControlArea) -> GitCall {
    let mut call = GitCall::new(UNQUOTED_PATHS).arg("diff").args(DIFF_FLAGS);
    if area == SourceControlArea::Index {
        call = call.arg("--cached");
    }
    call = call.arg("--").arg(path);
    if let Some(original) = original {
        call = call.arg(original);
    }
    call.stdout_limit(DIFF_LIMIT).timeout(READ_TIMEOUT)
}

fn untracked_diff(path: &str) -> GitCall {
    GitCall::new(UNQUOTED_PATHS)
        .args(["diff", "--no-index"])
        .args(DIFF_FLAGS)
        .args(["--", "/dev/null"])
        .arg(path)
        .accept_exit_codes(&[1])
        .stdout_limit(DIFF_LIMIT)
        .timeout(READ_TIMEOUT)
}
