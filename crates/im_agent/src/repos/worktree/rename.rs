// Path: crates/im_agent/src/repos/worktree/rename.rs
// Description: Renaming one worktree entry in place, never over anything that already exists

use std::collections::BTreeSet;
use std::io;
use std::path::Path;

use im_bundle::fs_atomic::rename_no_replace;
use tokio::fs;

use crate::error::{AgentError, MutationEffect};
use crate::source_control::ensure_within_root;

use super::destination::join_relative;
use super::entries::{
    conflict_error, destination_is_the_source, entry_not_found, existing_kind, normalize_entry,
};

/// Renames one entry inside its own folder and returns its new repo-relative
/// path.
///
/// A rename never replaces: there is no policy on this action, because the
/// gesture that produces it (typing a new name over the old one) carries no
/// answer to "and destroy what is already called that?". An occupied
/// destination is refused and named, and the user renames again or deletes
/// first. The one destination that is not a conflict is the entry itself,
/// reached by another spelling — the case-only rename a case-insensitive
/// filesystem reports as occupied.
///
/// Because a rename never replaces, the write itself is the no-replace
/// primitive, and the check above is only what lets it report the occupied
/// destination well. The one exception is that case-only rename: the
/// destination has been proved to *be* the source, so no primitive that
/// refuses an occupied destination could ever perform it, and it takes the
/// plain rename. Both are synchronous `std::fs` inside an async fn, exactly as
/// `source_control::discard::entries` claims an entry.
pub(super) async fn rename_entry(
    repo_root: &Path,
    path: &str,
    new_name: &str,
) -> Result<String, AgentError> {
    let source_rel = normalize_entry(path)?;
    ensure_valid_name(new_name)?;
    ensure_within_root(repo_root, &source_rel)?;
    let source = repo_root.join(&source_rel);
    if fs::symlink_metadata(&source).await.is_err() {
        return Err(entry_not_found(&source_rel));
    }

    let parent = source_rel
        .rsplit_once('/')
        .map_or("", |(parent, _)| parent);
    let dest_rel = join_relative(parent, new_name);
    ensure_within_root(repo_root, &dest_rel)?;
    let dest = repo_root.join(&dest_rel);

    let destination_is_itself = destination_is_the_source(&source, &dest).await;
    if !destination_is_itself && existing_kind(repo_root, &dest_rel).await?.is_some() {
        return Err(conflict_error(
            BTreeSet::from([dest_rel]),
            "Something is already called that in this folder",
        ));
    }

    let renamed = if destination_is_itself {
        std::fs::rename(&source, &dest)
    } else {
        rename_no_replace(&source, &dest)
    };
    renamed.map_err(|error| rename_failure(&dest_rel, &error))?;
    Ok(dest_rel)
}

/// A new name is one name, not a path: anything that could steer the rename
/// into another folder, out of the worktree, or into the repository's own Git
/// directory is refused before the entry is touched.
fn ensure_valid_name(new_name: &str) -> Result<(), AgentError> {
    let refusal = if new_name.trim().is_empty() {
        Some("it is empty")
    } else if new_name.contains('/') || new_name.contains('\\') || new_name.contains('\0') {
        Some("it is a path, not a name")
    } else if new_name == "." || new_name == ".." {
        Some("it names a folder rather than an entry in one")
    } else if new_name.eq_ignore_ascii_case(".git") {
        Some("it names the repository's own Git directory")
    } else {
        None
    };
    match refusal {
        None => Ok(()),
        Some(reason) => Err(AgentError::new(
            "ENTRY_INVALID_NAME",
            format!("Cannot rename to {new_name:?}: {reason}"),
        )),
    }
}

/// One entry, one rename: a failure here is the whole action failing, so the
/// worktree is exactly as it was and the effect says so.
///
/// `AlreadyExists` is a writer that took the destination between the check and
/// the rename; the no-replace primitive lost to it rather than destroying it,
/// so it is the same conflict the check would have reported. `Unsupported` is
/// a filesystem with no no-replace rename at all — WSL's mount of a Windows
/// drive — which no rename here may fall back past, and which `discard::claim`
/// answers the same way.
fn rename_failure(dest_rel: &str, error: &io::Error) -> AgentError {
    let failure = match error.kind() {
        io::ErrorKind::AlreadyExists => conflict_error(
            BTreeSet::from([dest_rel.to_string()]),
            "Something is already called that in this folder",
        ),
        io::ErrorKind::Unsupported => AgentError::new(
            "SOURCE_CONTROL_UNSUPPORTED_LAYOUT",
            "the filesystem cannot rename without replacing; serve Windows drives through the host agent",
        ),
        io::ErrorKind::CrossesDevices => AgentError::new(
            "SOURCE_CONTROL_UNSUPPORTED_LAYOUT",
            format!("Cannot rename to {dest_rel}: it would cross a volume boundary"),
        ),
        _ => AgentError::internal(format!("Failed to rename to {dest_rel}: {error}")),
    };
    failure.with_effect(MutationEffect::NotApplied)
}
