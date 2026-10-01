// Path: crates/im_agent/src/repos/import/sources.rs
// Description: Source translation, per-source validation, and the bounded walk that plans an import

use std::collections::VecDeque;
use std::io;
use std::path::{Path, PathBuf};

use tokio::fs;

use crate::error::AgentError;
use crate::repos::worktree::join_relative;
use crate::source_control::ensure_no_git_component;
use crate::staging::StageFileCancelToken;

use super::{cancelled_error, unsupported_source};

/// Bounds files and directories across the entire drop before any write.
pub const MAX_IMPORT_ENTRIES: usize = 10_000;

/// Planned directories precede their descendants, so writes need no unplanned parents.
pub(super) enum PlannedEntry {
    Dir { dest_rel: String },
    File { source: PathBuf, dest_rel: String },
}

impl PlannedEntry {
    pub(super) fn dest_rel(&self) -> &str {
        match self {
            Self::Dir { dest_rel } | Self::File { dest_rel, .. } => dest_rel,
        }
    }
}

/// The repo-relative root claimed by one source and its ordered descendants.
pub(super) struct PlannedSource {
    pub(super) dest_rel: String,
    pub(super) entries: Vec<PlannedEntry>,
}

/// Validates every source and expands directories into entries. Nothing here
/// writes: a refusal from this function proves the worktree is untouched.
pub(super) async fn plan_sources(
    sources: &[PathBuf],
    directory: &str,
    canonical_dest_dir: &Path,
    cancel: &StageFileCancelToken,
) -> Result<Vec<PlannedSource>, AgentError> {
    let mut planned = Vec::with_capacity(sources.len());
    let mut used = 0usize;

    for source in sources {
        if cancel.is_cancelled() {
            return Err(cancelled_error(&[]));
        }
        let source = source.as_path();
        let basename = source_basename(source)?;
        let canonical = classify_source(source).await?;
        ensure_not_a_container_of_the_destination(&canonical, canonical_dest_dir, &basename)?;

        let dest_rel = join_relative(directory, &basename);
        let entries = if canonical.is_dir {
            walk_directory(source, &dest_rel, &mut used, cancel).await?
        } else {
            used = charge_entry(used)?;
            vec![PlannedEntry::File {
                source: source.to_path_buf(),
                dest_rel: dest_rel.clone(),
            }]
        };
        planned.push(PlannedSource { dest_rel, entries });
    }

    Ok(planned)
}

/// Source paths have already been translated into the owning agent's namespace.
fn refuse(source: &Path, reason: &str) -> AgentError {
    unsupported_source(source.display(), reason)
}

fn source_basename(source: &Path) -> Result<String, AgentError> {
    let name = source
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .ok_or_else(|| refuse(source, "names no file or folder"))?;
    if name.is_empty() {
        return Err(refuse(source, "has an empty name"));
    }
    if name.contains('/') || name.contains('\\') || name.contains('\0') {
        return Err(refuse(source, "has a name a repo path cannot carry"));
    }
    ensure_no_git_component(&name)?;
    Ok(name)
}

struct ClassifiedSource {
    path: PathBuf,
    is_dir: bool,
}

/// Confirms the source exists, is not a symlink, and resolves it once so the
/// self-import checks compare real filesystem identities.
async fn classify_source(source: &Path) -> Result<ClassifiedSource, AgentError> {
    let metadata = match fs::symlink_metadata(source).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(AgentError::new(
                "IMPORT_SOURCE_NOT_FOUND",
                format!("Dropped source does not exist: {}", source.display()),
            ))
        }
        Err(error) => {
            return Err(AgentError::internal(format!(
                "Failed to read {}: {error}",
                source.display()
            )))
        }
    };
    if metadata.is_symlink() {
        return Err(refuse(source, "is a symbolic link"));
    }
    if !metadata.is_dir() && !metadata.is_file() {
        return Err(refuse(source, "is not a regular file or folder"));
    }
    let path = fs::canonicalize(source).await.map_err(|error| {
        AgentError::internal(format!("Failed to resolve {}: {error}", source.display()))
    })?;
    Ok(ClassifiedSource {
        path,
        is_dir: metadata.is_dir(),
    })
}

/// Refuses self-import and sources containing the destination before expansion.
fn ensure_not_a_container_of_the_destination(
    source: &ClassifiedSource,
    canonical_dest_dir: &Path,
    basename: &str,
) -> Result<(), AgentError> {
    if canonical_dest_dir.starts_with(&source.path) {
        return Err(refuse(
            &source.path,
            "is the destination folder or contains it",
        ));
    }
    if source.path == canonical_dest_dir.join(basename) {
        return Err(refuse(&source.path, "is already at the destination"));
    }
    Ok(())
}

/// Refuses Git control entries before type dispatch; ordinary symlinks are skipped.
async fn walk_directory(
    source_root: &Path,
    dest_root_rel: &str,
    used: &mut usize,
    cancel: &StageFileCancelToken,
) -> Result<Vec<PlannedEntry>, AgentError> {
    *used = charge_entry(*used)?;
    let mut entries = vec![PlannedEntry::Dir {
        dest_rel: dest_root_rel.to_string(),
    }];
    let mut queue = VecDeque::from([(source_root.to_path_buf(), dest_root_rel.to_string())]);

    while let Some((dir, dir_dest_rel)) = queue.pop_front() {
        if cancel.is_cancelled() {
            return Err(cancelled_error(&[]));
        }
        let mut read_dir = fs::read_dir(&dir).await.map_err(|error| {
            AgentError::internal(format!("Failed to read {}: {error}", dir.display()))
        })?;
        while let Some(entry) = read_dir.next_entry().await.map_err(|error| {
            AgentError::internal(format!("Failed to read {}: {error}", dir.display()))
        })? {
            let name = entry.file_name().to_string_lossy().to_string();
            let dest_rel = join_relative(&dir_dest_rel, &name);
            ensure_no_git_component(&dest_rel)
                .map_err(|_| git_inside_drop(source_root, dest_root_rel, &dest_rel))?;
            let file_type = entry.file_type().await.map_err(|error| {
                AgentError::internal(format!("Failed to read {}: {error}", dir.display()))
            })?;
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                *used = charge_entry(*used)?;
                entries.push(PlannedEntry::Dir {
                    dest_rel: dest_rel.clone(),
                });
                queue.push_back((entry.path(), dest_rel));
            } else if file_type.is_file() {
                *used = charge_entry(*used)?;
                entries.push(PlannedEntry::File {
                    source: entry.path(),
                    dest_rel,
                });
            }
        }
    }

    Ok(entries)
}

/// Names the dropped root and the Git control entry relative to that root.
fn git_inside_drop(source_root: &Path, dest_root_rel: &str, dest_rel: &str) -> AgentError {
    let inside = dest_rel
        .strip_prefix(dest_root_rel)
        .map_or(dest_rel, |rest| rest.trim_start_matches('/'));
    AgentError::new(
        "INVALID_PATH",
        format!(
            "Refusing {}: it contains Git control metadata at {inside}",
            source_root.display()
        ),
    )
}

fn charge_entry(used: usize) -> Result<usize, AgentError> {
    if used >= MAX_IMPORT_ENTRIES {
        return Err(AgentError::new(
            "IMPORT_TOO_LARGE",
            format!("An import may carry at most {MAX_IMPORT_ENTRIES} files and folders"),
        ));
    }
    Ok(used + 1)
}
