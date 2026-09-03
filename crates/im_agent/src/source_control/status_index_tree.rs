// Path: crates/im_agent/src/source_control/status_index_tree.rs
// Description: Read-only identity of the whole-repository index (`git write-tree` without writing)

use std::path::{Path, PathBuf};

use im_bundle::git::{index_tree_sha, BundleCancelToken, IndexTreeError};

use crate::error::AgentError;

use super::runner::{self, GitCall, READ_TIMEOUT};

const INDEX_LISTING_LIMIT: usize = 32 * 1024 * 1024;

/// The tree id a commit of the current index would carry, computed from
/// `git ls-files --stage -z` without writing an object. The listing is taken at
/// the Git top level so a configured subdirectory root still sees the whole
/// index a commit would carry.
///
/// An index with unmerged entries has no candidate tree at all; that is
/// reported as an empty identity rather than a tree over the resolved subset,
/// which would name a commit nobody could make. A listing that outgrew its
/// bound is an error: a tree computed from half an index is a false identity,
/// and this value is a commit precondition.
pub(super) async fn capture_index_tree_sha(
    repo_root: &Path,
    prefix: &[u8],
    cancel_token: Option<BundleCancelToken>,
) -> Result<String, AgentError> {
    let top_level = git_top_level(repo_root, prefix)?;
    let call = GitCall::new(["ls-files", "--stage", "-z", "--full-name"])
        .stdout_limit(INDEX_LISTING_LIMIT)
        .timeout(READ_TIMEOUT);
    let output = runner::run_read(&top_level, call, cancel_token).await?;
    if output.stdout_truncated {
        return Err(AgentError::new(
            "GIT_COMMAND_FAILED",
            "The index listing exceeded its output bound, so the index identity is unknown",
        ));
    }
    match index_tree_sha(&output.stdout) {
        Ok(sha) => Ok(sha),
        Err(IndexTreeError::Unmerged) => Ok(String::new()),
        Err(IndexTreeError::Malformed) => Err(AgentError::new(
            "GIT_COMMAND_FAILED",
            "The index listing could not be parsed, so the index identity is unknown",
        )),
    }
}

/// The Git top level, derived by dropping the repo prefix's components from the
/// configured root; `git rev-parse --show-prefix` has already been paid for.
pub(super) fn git_top_level(repo_root: &Path, prefix: &[u8]) -> Result<PathBuf, AgentError> {
    let depth = prefix
        .split(|byte| *byte == b'/')
        .filter(|component| !component.is_empty())
        .count();
    let mut top_level = repo_root.to_path_buf();
    for _ in 0..depth {
        if !top_level.pop() {
            return Err(AgentError::internal(format!(
                "Repo root {} is shallower than its Git prefix",
                repo_root.display()
            )));
        }
    }
    Ok(top_level)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::git_top_level;

    #[test]
    fn the_prefix_depth_is_dropped_from_the_configured_root() {
        let root = Path::new("/home/dev/repo/sub/deeper");
        assert_eq!(
            git_top_level(root, b"sub/deeper/").expect("top level"),
            PathBuf::from("/home/dev/repo")
        );
        assert_eq!(git_top_level(root, b"").expect("top level"), root);
    }
}
