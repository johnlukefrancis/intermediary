// Path: crates/im_agent/src/repos/import/copy.rs
// Description: The import conflict pre-pass and the policy-specific copy that writes into the worktree

use std::collections::BTreeSet;
use std::io;
use std::path::Path;

use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::error::{AgentError, MutationEffect};
use crate::protocol::{ImportConflictPolicy, ImportedFile};
use crate::repos::worktree::{conflict_error, existing_kind, kind_mismatch_error};
use crate::source_control::ensure_within_root;
use crate::staging::{temp_path_for, StageFileCancelToken};

use super::sources::{PlannedEntry, PlannedSource};
use super::cancelled_error;

/// Refuses the whole drop before anything is written. Three proofs are owed
/// here, and only here: that no destination resolves outside the worktree
/// through a directory component that is already a symlink; that no planned
/// entry would land on an existing path of the other kind (a file over a
/// folder or a folder over a file — no policy can replace across kinds, so it
/// is refused under both); and that every file collision live right now was
/// one the user authorized replacing. A planned folder over an existing folder
/// is a merge, never a conflict, so the conflict list names files — the unit
/// an authorization is written in.
///
/// The collision set is computed the same way under both policies, and the
/// refusal reports all of it: a user who authorized `app/a.txt` and meets a
/// second file at `app/b.txt` is shown the whole fresh list, not the one new
/// name, because that list is what the next authorization has to answer.
/// Under `Refuse` nothing is ever authorized, so any collision refuses — which
/// is exactly the older behaviour, reached by the same route.
pub(super) async fn ensure_writable(
    repo_root: &Path,
    planned: &[PlannedSource],
    policy: &ImportConflictPolicy,
) -> Result<(), AgentError> {
    let mut conflicts = BTreeSet::new();
    let mut mismatches = BTreeSet::new();
    for source in planned {
        for entry in &source.entries {
            ensure_within_root(repo_root, entry.dest_rel())?;
            let Some(existing_is_dir) = existing_kind(repo_root, entry.dest_rel()).await? else {
                continue;
            };
            match (entry, existing_is_dir) {
                (PlannedEntry::Dir { .. }, true) => {}
                (PlannedEntry::File { .. }, false) => {
                    conflicts.insert(entry.dest_rel().to_string());
                }
                _ => {
                    mismatches.insert(entry.dest_rel().to_string());
                }
            }
        }
    }
    if !mismatches.is_empty() {
        return Err(kind_mismatch_error(
            mismatches,
            "A dropped file would land on an existing folder, or a folder on an existing file",
        ));
    }
    if conflicts
        .iter()
        .all(|dest_rel| policy.replaces(dest_rel))
    {
        return Ok(());
    }
    Err(conflict_error(
        conflicts,
        "Some dropped files already exist in this folder",
    ))
}

/// Writes the plan. Every failure past this point carries what already landed
/// (`details.imported`) and the `unknown` effect, because a partially applied
/// import is exactly the state only a fresh read can describe.
pub(super) async fn write_planned(
    repo_root: &Path,
    planned: &[PlannedSource],
    policy: &ImportConflictPolicy,
    cancel: &StageFileCancelToken,
) -> Result<Vec<ImportedFile>, AgentError> {
    let mut imported: Vec<ImportedFile> = Vec::new();

    for source in planned {
        for entry in &source.entries {
            if cancel.is_cancelled() {
                return Err(cancelled_error(&imported));
            }
            match entry {
                PlannedEntry::Dir { dest_rel } => {
                    fs::create_dir_all(repo_root.join(dest_rel))
                        .await
                        .map_err(|error| write_failure(&imported, dest_rel, error))?;
                }
                PlannedEntry::File { source, dest_rel } => {
                    // Asked per destination, never once for the drop: only the
                    // paths the user was shown and authorized may be replaced.
                    let replacing = policy.replaces(dest_rel);
                    let bytes = copy_file(source, &repo_root.join(dest_rel), replacing)
                        .await
                        .map_err(|error| write_failure(&imported, dest_rel, error))?;
                    imported.push(ImportedFile {
                        path: dest_rel.clone(),
                        bytes,
                    });
                }
            }
        }
    }

    Ok(imported)
}

/// Copies one file in the mode this destination earned.
///
/// An unauthorized destination is written directly with `create_new`, so there
/// is no temporary file to leave behind and a writer that won the race between
/// the pre-pass and here loses to `AlreadyExists` instead of being
/// overwritten — the filesystem, not a check, is what refuses it. An
/// authorized one writes a sibling temp file and renames it over the
/// destination, so a reader of that path sees either the old bytes or all of
/// the new ones. Both paths remove what they created on every failure.
async fn copy_file(
    source: &Path,
    destination: &Path,
    replacing: bool,
) -> Result<u64, io::Error> {
    let mut reader = fs::File::open(source).await?;
    let write_path = if replacing {
        temp_path_for(destination)
    } else {
        destination.to_path_buf()
    };

    let mut writer = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&write_path)
        .await?;

    let copied = match tokio::io::copy(&mut reader, &mut writer).await {
        Ok(copied) => copied,
        Err(error) => {
            let _ = fs::remove_file(&write_path).await;
            return Err(error);
        }
    };
    if let Err(error) = writer.flush().await {
        let _ = fs::remove_file(&write_path).await;
        return Err(error);
    }
    drop(writer);

    if replacing {
        if let Err(error) = fs::rename(&write_path, destination).await {
            let _ = fs::remove_file(&write_path).await;
            return Err(error);
        }
    }
    Ok(copied)
}

/// A failure with bytes already on disk. `AlreadyExists` is the racing-writer
/// case an unauthorized destination's copy is built to lose: it is a conflict,
/// not an internal fault, and it names the path it lost. Its effect is `notApplied` only while
/// nothing has landed yet — once a file is in the worktree, no site here can
/// claim the drop did nothing.
fn write_failure(imported: &[ImportedFile], dest_rel: &str, error: io::Error) -> AgentError {
    let effect = if imported.is_empty() {
        MutationEffect::NotApplied
    } else {
        MutationEffect::Unknown
    };
    if error.kind() == io::ErrorKind::AlreadyExists {
        return conflict_error(
            BTreeSet::from([dest_rel.to_string()]),
            "Another writer created this path during the import",
        )
        .with_details(serde_json::json!({ "conflicts": [dest_rel], "imported": imported }))
        .with_effect(effect);
    }
    AgentError::internal(format!("Failed to import {dest_rel}: {error}"))
        .with_details(serde_json::json!({ "imported": imported }))
        .with_effect(MutationEffect::Unknown)
}
