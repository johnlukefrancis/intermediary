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
use crate::source_control::{ensure_no_git_component, ensure_within_root};
use crate::staging::{temp_path_for, StageFileCancelToken};

use super::cancelled_error;
use super::sources::{PlannedEntry, PlannedSource};

/// Validates every destination before writes; Replace cannot authorize Git metadata,
/// an escaping path, a kind mismatch, or collisions outside the reviewed set.
pub(super) async fn ensure_writable(
    repo_root: &Path,
    planned: &[PlannedSource],
    policy: &ImportConflictPolicy,
) -> Result<(), AgentError> {
    let mut conflicts = BTreeSet::new();
    let mut mismatches = BTreeSet::new();
    for source in planned {
        for entry in &source.entries {
            ensure_no_git_component(entry.dest_rel())?;
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
    if conflicts.iter().all(|dest_rel| policy.replaces(dest_rel)) {
        return Ok(());
    }
    Err(conflict_error(
        conflicts,
        "Some dropped files already exist in this folder",
    ))
}

/// Write failures carry landed entries and an effect requiring reconciliation.
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

/// create_new refuses racing writers; authorized replacement publishes a complete temp file.
async fn copy_file(source: &Path, destination: &Path, replacing: bool) -> Result<u64, io::Error> {
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

/// A racing-writer conflict is notApplied only while no files have landed.
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
