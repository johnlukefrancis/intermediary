// Path: crates/im_agent/src/repos/worktree/entries.rs
// Description: The repo-relative entry path law every worktree action shares, and the refusals it raises

use std::collections::BTreeSet;
use std::io;
use std::path::Path;

use tokio::fs;

use crate::error::AgentError;
use crate::source_control::{ensure_no_git_component, normalize_path};

/// One UI-supplied entry path, validated and normalized to the slash form the
/// rest of the wire uses.
///
/// An entry names one thing the user selected, so the worktree root itself
/// (`""` and `"."`) is not an entry and `normalize_path` already refuses both,
/// along with traversal, absolute paths, backslashes, and NUL. What is added
/// here is the `.git` refusal: no worktree action may reach the repository's
/// own directory at any depth, whichever side of the operation names it.
pub(super) fn normalize_entry(path: &str) -> Result<String, AgentError> {
    let normalized = normalize_path(path)?;
    ensure_no_git_component(&normalized)?;
    Ok(normalized)
}

/// `Some(is_dir)` when something already sits at the destination, `None` when
/// the path is free. A symlink counts as a file: a replacing rename goes over
/// it and never follows it.
pub(crate) async fn existing_kind(
    repo_root: &Path,
    dest_rel: &str,
) -> Result<Option<bool>, AgentError> {
    match fs::symlink_metadata(repo_root.join(dest_rel)).await {
        Ok(metadata) => Ok(Some(metadata.is_dir())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(AgentError::internal(format!(
            "Failed to check {dest_rel}: {error}"
        ))),
    }
}

/// Whether the destination path *is* the source, reached by another name.
///
/// This is the case-only rename on a case-insensitive filesystem: `Notes.md`
/// to `notes.md` on NTFS or drvfs finds the destination occupied by the very
/// file being renamed. Refusing that as a conflict would make the rename
/// impossible, so identity is asked of the filesystem rather than of the two
/// strings. A symlink at the destination is never the source: it resolves to
/// the same file while being a different entry, and replacing it would destroy
/// the link.
pub(super) async fn destination_is_the_source(source: &Path, dest: &Path) -> bool {
    match fs::symlink_metadata(dest).await {
        Ok(metadata) if metadata.is_symlink() => return false,
        Ok(_) => {}
        Err(_) => return false,
    }
    match (fs::canonicalize(dest).await, fs::canonicalize(source).await) {
        (Ok(dest), Ok(source)) => dest == source,
        _ => false,
    }
}

/// The one destination-is-taken refusal, shared by every path that writes the
/// worktree without Git. `details.conflicts` names the repo-relative paths
/// that collided so the UI can offer the replace the user was not asked for.
pub(crate) fn conflict_error(conflicts: BTreeSet<String>, message: &str) -> AgentError {
    let conflicts: Vec<String> = conflicts.into_iter().collect();
    AgentError::new("ENTRY_CONFLICT", message)
        .with_details(serde_json::json!({ "conflicts": conflicts }))
}

/// A file would land on an existing folder, or a folder on an existing file.
/// No policy can replace across kinds — a replace was authorized for bytes,
/// never for a tree — so this is refused under both.
pub(crate) fn kind_mismatch_error(conflicts: BTreeSet<String>, message: &str) -> AgentError {
    let conflicts: Vec<String> = conflicts.into_iter().collect();
    AgentError::new("ENTRY_KIND_MISMATCH", message)
        .with_details(serde_json::json!({ "conflicts": conflicts }))
}

pub(super) fn entry_not_found(path: &str) -> AgentError {
    AgentError::new("ENTRY_NOT_FOUND", format!("{path} no longer exists"))
}

#[cfg(test)]
mod tests {
    use super::normalize_entry;

    #[test]
    fn an_entry_is_normalized_and_never_names_the_git_directory() {
        assert_eq!(normalize_entry("./app//a.txt").expect("entry"), "app/a.txt");
        for bad in ["", ".", "../x", "/abs", ".git", "app/.git/config", "a/.GIT/x"] {
            assert_eq!(
                normalize_entry(bad).expect_err(bad).code(),
                "INVALID_PATH",
                "{bad}"
            );
        }
    }

}
