// Path: crates/im_bundle/src/git_capture/index.rs
// Description: Bounded capture of the candidate index tree identity

use std::ffi::OsString;

use crate::cancel::BundleCancelToken;
use crate::error::Result;

use super::command::run_git;
use super::diff::common_git_config_args;
use super::index_tree::{index_tree_sha, IndexTreeError};
use super::{GitCaptureConfig, GitCaptureIssue};

const INDEX_LISTING_LIMIT: usize = 32 * 1024 * 1024;

/// Returns the tree id the whole-repository index would commit as. Only the
/// hash crosses into evidence; the listing itself never leaves this process.
pub(crate) fn capture_index_tree_sha(
    config: &GitCaptureConfig,
    cancel_token: Option<&BundleCancelToken>,
) -> Result<std::result::Result<String, GitCaptureIssue>> {
    // Deliberately not `--literal-pathspecs`: `:/` is our own top-level magic
    // so the listing covers the whole repository from any bundle root.
    let mut args = common_git_config_args();
    args.extend([
        OsString::from("ls-files"),
        OsString::from("--stage"),
        OsString::from("-z"),
        OsString::from("--full-name"),
        OsString::from("--"),
        OsString::from(":/"),
    ]);
    let output = run_git(
        &config.executable,
        &config.repo_root,
        &args,
        INDEX_LISTING_LIMIT,
        config.command_timeout,
        cancel_token,
    )?;
    let output = match output {
        Ok(output) => output,
        Err(_) => return Ok(Err(GitCaptureIssue::new(
            "indexUnavailable",
            None,
            "The candidate index listing could not be captured; candidateIndexTreeSha is absent.",
        ))),
    };
    if output.stdout_truncated {
        return Ok(Err(GitCaptureIssue::new(
            "outputTruncated",
            None,
            "The candidate index listing exceeded its safety bound; candidateIndexTreeSha is absent.",
        )));
    }
    Ok(match index_tree_sha(&output.stdout) {
        Ok(sha) => Ok(sha),
        Err(IndexTreeError::Unmerged) => Err(GitCaptureIssue::new(
            "indexUnmerged",
            None,
            "The index holds unmerged entries, so no candidate tree exists yet.",
        )),
        Err(IndexTreeError::Malformed) => Err(GitCaptureIssue::new(
            "indexParseFailure",
            None,
            "The candidate index listing could not be parsed safely.",
        )),
    })
}
