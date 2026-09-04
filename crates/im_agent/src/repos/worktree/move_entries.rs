// Path: crates/im_agent/src/repos/worktree/move_entries.rs
// Description: Moving selected worktree entries into one destination folder, refused whole before the first rename

use std::collections::BTreeSet;
use std::io;
use std::path::Path;

use im_bundle::fs_atomic::rename_no_replace;
use tokio::fs;

use crate::error::{AgentError, MutationEffect};
use crate::protocol::ImportConflictPolicy;
use crate::repos::normalize_directory_path;
use crate::source_control::{ensure_no_git_component, ensure_within_root};

use super::destination::{
    ensure_distinct_destinations, join_relative, normalize_authorization, resolve_destination,
};
use super::entries::{
    conflict_error, destination_is_the_source, entry_not_found, existing_kind, kind_mismatch_error,
    normalize_entry,
};

/// One entry the move will rename, resolved. `already_there` is the entry the
/// user dragged into the folder it is already in: nothing to do, but it is
/// still one of the paths the action produced, so it is reported like the
/// rest rather than silently dropped.
struct PlannedMove {
    source_rel: String,
    dest_rel: String,
    source_is_dir: bool,
    already_there: bool,
}

/// Moves `paths` into `<repo_root>/<directory>` and returns the destination
/// path of each, in the order given.
///
/// The order below is the contract, and it is the import's: resolve the
/// destination, plan and validate every entry, refuse the whole action, and
/// only then rename. Any error raised before the first rename proves the
/// worktree is untouched.
///
/// A folder never merges here. A move that landed a folder on an existing
/// folder of the same name would have to either replace the existing tree —
/// destroying files nobody selected — or merge two histories the user never
/// asked to combine, so it is refused under both policies. Only file over
/// file answers to `policy`, and then only at the destinations that policy
/// names.
///
/// The renames are synchronous `std::fs` inside an async fn, exactly as
/// `source_control::discard::entries` claims an entry: a rename inside one
/// filesystem is microseconds, and the no-replace primitive an unauthorized
/// destination is written with has no async form to call.
pub(super) async fn move_entries(
    repo_root: &Path,
    paths: &[String],
    directory: &str,
    policy: &ImportConflictPolicy,
) -> Result<Vec<String>, AgentError> {
    if paths.is_empty() {
        return Err(AgentError::new("INVALID_PATH", "No paths given"));
    }
    let directory = normalize_directory_path(directory)?;
    ensure_no_git_component(&directory)?;
    let policy = normalize_authorization(policy)?;
    resolve_destination(repo_root, &directory).await?;

    let mut planned = Vec::with_capacity(paths.len());
    for path in paths {
        planned.push(plan_one(repo_root, path, &directory).await?);
    }
    ensure_distinct_destinations(planned.iter().map(|entry| entry.dest_rel.as_str()))?;
    ensure_writable(repo_root, &planned, &policy).await?;

    let mut applied: Vec<String> = Vec::with_capacity(planned.len());
    for entry in &planned {
        if !entry.already_there {
            rename_one(repo_root, entry, &policy)
                .map_err(|error| move_failure(&applied, &entry.dest_rel, &error))?;
        }
        applied.push(entry.dest_rel.clone());
    }
    Ok(applied)
}

/// The one rename, in the mode this destination earned. An authorized
/// destination gets the replacing rename the user asked for; every other
/// destination gets the primitive that cannot replace, so anything that
/// appeared there after the pre-pass — including through a case alias no
/// string comparison here can see — is refused by the filesystem, not
/// destroyed.
fn rename_one(
    repo_root: &Path,
    entry: &PlannedMove,
    policy: &ImportConflictPolicy,
) -> io::Result<()> {
    let source = repo_root.join(&entry.source_rel);
    let dest = repo_root.join(&entry.dest_rel);
    if policy.replaces(&entry.dest_rel) {
        return std::fs::rename(&source, &dest);
    }
    rename_no_replace(&source, &dest)
}

async fn plan_one(
    repo_root: &Path,
    path: &str,
    directory: &str,
) -> Result<PlannedMove, AgentError> {
    let source_rel = normalize_entry(path)?;
    ensure_within_root(repo_root, &source_rel)?;
    let source = repo_root.join(&source_rel);
    let source_is_dir = match fs::symlink_metadata(&source).await {
        Ok(metadata) => metadata.is_dir(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(entry_not_found(&source_rel))
        }
        Err(error) => {
            return Err(AgentError::internal(format!(
                "Failed to check {source_rel}: {error}"
            )))
        }
    };
    ensure_not_inside_itself(&source_rel, directory)?;

    let name = source_rel.rsplit('/').next().unwrap_or(source_rel.as_str());
    let dest_rel = join_relative(directory, name);
    ensure_within_root(repo_root, &dest_rel)?;
    let dest = repo_root.join(&dest_rel);
    let already_there =
        dest_rel == source_rel || destination_is_the_source(&source, &dest).await;

    Ok(PlannedMove {
        source_rel,
        dest_rel,
        source_is_dir,
        already_there,
    })
}

/// A folder cannot be moved into itself or into anything below it: the
/// destination would travel with the source and the rename either fails
/// obscurely or loses the tree. Compared as repo-relative strings, which is
/// exactly what both sides already are.
fn ensure_not_inside_itself(source_rel: &str, directory: &str) -> Result<(), AgentError> {
    if directory == source_rel || directory.starts_with(&format!("{source_rel}/")) {
        return Err(AgentError::new(
            "INVALID_PATH",
            format!("Refusing to move {source_rel} into itself"),
        ));
    }
    Ok(())
}

/// Refuses the whole move before anything is renamed. Kind mismatches are
/// reported first, then folders over folders, then files over files that the
/// policy did not authorize: each of those is a different thing the user can
/// do about it, and the first two no policy can lift.
///
/// The file collisions are gathered under both policies and the refusal
/// reports all of them, so a `Replace` that met a destination it never
/// authorized hands back the whole list the next authorization must answer.
async fn ensure_writable(
    repo_root: &Path,
    planned: &[PlannedMove],
    policy: &ImportConflictPolicy,
) -> Result<(), AgentError> {
    let mut mismatches = BTreeSet::new();
    let mut folders = BTreeSet::new();
    let mut files = BTreeSet::new();
    for entry in planned {
        if entry.already_there {
            continue;
        }
        let Some(existing_is_dir) = existing_kind(repo_root, &entry.dest_rel).await? else {
            continue;
        };
        match (entry.source_is_dir, existing_is_dir) {
            (true, true) => {
                folders.insert(entry.dest_rel.clone());
            }
            (false, false) => {
                files.insert(entry.dest_rel.clone());
            }
            _ => {
                mismatches.insert(entry.dest_rel.clone());
            }
        }
    }
    if !mismatches.is_empty() {
        return Err(kind_mismatch_error(
            mismatches,
            "An item would land on something of the other kind, which no policy replaces",
        ));
    }
    if !folders.is_empty() {
        return Err(conflict_error(
            folders,
            "A folder cannot be moved onto an existing folder of the same name",
        ));
    }
    if files.iter().all(|dest_rel| policy.replaces(dest_rel)) {
        return Ok(());
    }
    Err(conflict_error(
        files,
        "Some of the selected items already exist in the destination folder",
    ))
}

/// A rename that failed, classified by what the filesystem actually answered.
///
/// `AlreadyExists` is the no-replace primitive doing its job: something the
/// user never authorized replacing sits at that destination, which is a
/// conflict naming the path, not an internal fault. `Unsupported` is a
/// filesystem with no no-replace rename at all — WSL's mount of a Windows
/// drive — answered exactly as `discard::claim` answers it: a layout to
/// change, not a call to retry. Cross-volume is the other such layout.
///
/// The effect is `notApplied` only while nothing has moved yet — once an
/// earlier entry has landed, no site here can claim the action did nothing,
/// and `details.applied` names what did land.
pub(super) fn move_failure(applied: &[String], dest_rel: &str, error: &io::Error) -> AgentError {
    let failure = match error.kind() {
        io::ErrorKind::AlreadyExists => conflict_error(
            BTreeSet::from([dest_rel.to_string()]),
            "Another writer created this path during the move",
        ),
        io::ErrorKind::Unsupported => AgentError::new(
            "SOURCE_CONTROL_UNSUPPORTED_LAYOUT",
            "the filesystem cannot rename without replacing; serve Windows drives through the host agent",
        ),
        io::ErrorKind::CrossesDevices => AgentError::new(
            "SOURCE_CONTROL_UNSUPPORTED_LAYOUT",
            format!("Cannot move {dest_rel}: it would cross a volume boundary"),
        ),
        _ => AgentError::internal(format!("Failed to move {dest_rel}: {error}")),
    };
    if applied.is_empty() {
        return failure.with_effect(MutationEffect::NotApplied);
    }
    let details = if error.kind() == io::ErrorKind::AlreadyExists {
        serde_json::json!({ "conflicts": [dest_rel], "applied": applied })
    } else {
        serde_json::json!({ "applied": applied })
    };
    failure
        .with_details(details)
        .with_effect(MutationEffect::Unknown)
}
