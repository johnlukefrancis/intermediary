// Path: crates/im_bundle/src/git_capture/prefix.rs
// Description: Shared bounded capture of the Git repository prefix for a configured root

use std::ffi::OsString;
use std::path::Path;
use std::time::Duration;

use crate::cancel::BundleCancelToken;
use crate::error::Result;

use super::command::{run_git, GitCommandFailure};
use super::diff::common_git_args;
use super::discovery::trim_line_ending;

const PREFIX_LIMIT: usize = 1024 * 1024;

/// The path of `repo_root` relative to the Git top level, as raw bytes with a
/// trailing slash (empty when the root is the top level itself). Porcelain
/// paths are top-level-relative, so every consumer strips this prefix.
#[derive(Debug, Clone)]
pub struct RepoPrefixCapture {
    pub prefix: Vec<u8>,
    pub truncated: bool,
}

pub fn capture_repo_prefix(
    executable: &Path,
    repo_root: &Path,
    timeout: Duration,
    cancel_token: Option<&BundleCancelToken>,
) -> Result<std::result::Result<RepoPrefixCapture, GitCommandFailure>> {
    let mut args = common_git_args();
    args.extend([OsString::from("rev-parse"), OsString::from("--show-prefix")]);
    let output = run_git(executable, repo_root, &args, PREFIX_LIMIT, timeout, cancel_token)?;
    Ok(output.map(|output| RepoPrefixCapture {
        prefix: trim_line_ending(output.stdout),
        truncated: output.stdout_truncated,
    }))
}
