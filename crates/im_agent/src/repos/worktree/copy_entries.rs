// Path: crates/im_agent/src/repos/worktree/copy_entries.rs
// Description: Copying selected worktree entries into one destination folder through the import writer

use std::path::Path;

use tokio::fs;

use crate::error::AgentError;
use crate::protocol::ImportConflictPolicy;
use crate::repos::import::import_resolved;
use crate::repos::normalize_directory_path;
use crate::source_control::{ensure_no_git_component, ensure_within_root};
use crate::staging::StageFileCancelToken;

use super::destination::join_relative;
use super::entries::{entry_not_found, normalize_entry};

/// Copies `paths` into `<repo_root>/<directory>` and returns the destination
/// path of each, in the order given.
///
/// A copy inside the worktree is the same write an external drop performs —
/// same conflict pre-pass, same merge rule for folders, same partial-failure
/// reporting — with one thing removed: there is nothing to translate, because
/// the sources are already repo-relative entries in this agent's namespace.
/// So the entries are validated here and the resolved paths are handed to the
/// import writer rather than a second copier being written beside it.
///
/// The destination paths are computed here rather than read back from the
/// import: the import answers with every file it wrote (a folder's whole
/// contents), and what this action produced is one path per selected entry.
pub(super) async fn copy_entries(
    repo_root: &Path,
    paths: &[String],
    directory: &str,
    policy: &ImportConflictPolicy,
    cancel: &StageFileCancelToken,
) -> Result<Vec<String>, AgentError> {
    if paths.is_empty() {
        return Err(AgentError::new("INVALID_PATH", "No paths given"));
    }
    let directory = normalize_directory_path(directory)?;
    ensure_no_git_component(&directory)?;

    let mut sources = Vec::with_capacity(paths.len());
    let mut entries = Vec::with_capacity(paths.len());
    for path in paths {
        let source_rel = normalize_entry(path)?;
        ensure_within_root(repo_root, &source_rel)?;
        let source = repo_root.join(&source_rel);
        if fs::symlink_metadata(&source).await.is_err() {
            return Err(entry_not_found(&source_rel));
        }
        let name = source_rel.rsplit('/').next().unwrap_or(source_rel.as_str());
        entries.push(join_relative(&directory, name));
        sources.push(source);
    }

    import_resolved(repo_root, &directory, sources, policy, cancel).await?;
    Ok(entries)
}
