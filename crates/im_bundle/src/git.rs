// Path: crates/im_bundle/src/git.rs
// Description: Public Git primitives shared by bundle evidence capture and agent source control

//! One owner for running Git: the bounded, cancellable runner, the strict
//! porcelain-v2 parser, byte-exact path transport, the shared argument
//! profile, and repository-prefix capture. Bundle evidence and the agents'
//! source-control feature both build on these; neither keeps a second copy.

pub use crate::cancel::BundleCancelToken;
pub use crate::git_capture::command::{
    run_git, run_git_with_input, GitCommandFailure, GitCommandFailureKind, GitCommandOutput,
    KillPolicy,
};
pub use crate::git_capture::command_tree::terminate_git_process_trees;
pub use crate::git_capture::diff::{common_git_args, common_git_config_args};
pub use crate::git_capture::discovery::trim_line_ending;
pub use crate::git_capture::index_tree::{index_tree_sha, IndexTreeError};
pub use crate::git_capture::path::{
    bytes_to_path, display_ref, path_to_bytes, strip_repo_prefix, GitPath,
};
pub use crate::git_capture::porcelain::{parse_porcelain, PorcelainStatus, StatusRecord};
pub use crate::git_capture::prefix::{capture_repo_prefix, RepoPrefixCapture};
