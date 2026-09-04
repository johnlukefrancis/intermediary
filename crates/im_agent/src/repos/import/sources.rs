// Path: crates/im_agent/src/repos/import/sources.rs
// Description: Source translation, per-source validation, and the bounded walk that plans an import

use std::collections::VecDeque;
use std::io;
use std::path::{Path, PathBuf};

use tokio::fs;

use crate::error::AgentError;
use crate::repos::worktree::join_relative;
use crate::staging::StageFileCancelToken;

use super::{cancelled_error, unsupported_source};

/// The ceiling on one drop, counted in files *and* directories across every
/// source. It bounds the plan before a single byte moves, so a folder dropped
/// by mistake is refused rather than half-copied.
pub const MAX_IMPORT_ENTRIES: usize = 10_000;

/// One thing the import will write, in the order it must be written: a
/// directory always appears before anything inside it, so the copy never has
/// to create a parent it was not told about.
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

/// One dropped source, resolved. `dest_rel` is the single repo-relative path
/// the source claims — `<directory>/<basename>` — and is the unit both the
/// conflict pre-pass and duplicate detection compare.
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

/// This module's refusals name the source by its resolved path: by the time
/// anything here speaks, the delivered string has already been translated into
/// this agent's namespace, and quoting the untranslated form would name a path
/// that does not exist here.
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
    if name.eq_ignore_ascii_case(".git") {
        return Err(refuse(source, "is a Git directory"));
    }
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

/// Refuses a source that would import itself: the destination directory is the
/// source, sits inside it, or the source is exactly the file the copy would
/// write. Each of those either loops forever or destroys the source.
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

/// Breadth-first expansion of one directory source. Symlinked entries inside
/// are skipped exactly as the directory listing skips them, so an import never
/// follows a link out of the tree the user dropped.
///
/// A Git directory anywhere inside refuses the whole drop. Copying one into a
/// worktree plants a second repository the user never asked for, and this is
/// still planning, so the refusal proves nothing was written — the same proof
/// the top-level basename refusal gives, extended to every depth the walk can
/// reach.
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
            let file_type = entry.file_type().await.map_err(|error| {
                AgentError::internal(format!("Failed to read {}: {error}", dir.display()))
            })?;
            if file_type.is_symlink() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let dest_rel = join_relative(&dir_dest_rel, &name);
            if file_type.is_dir() {
                if name.eq_ignore_ascii_case(".git") {
                    return Err(git_inside_drop(source_root, dest_root_rel, &dest_rel));
                }
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

/// Names the folder the user dropped and where inside it the Git directory
/// sits, because that folder is what they have to fix; `dest_rel` is stripped
/// back to its path within the drop, which is the only part they can act on.
fn git_inside_drop(source_root: &Path, dest_root_rel: &str, dest_rel: &str) -> AgentError {
    let inside = dest_rel
        .strip_prefix(dest_root_rel)
        .map_or(dest_rel, |rest| rest.trim_start_matches('/'));
    AgentError::new(
        "INVALID_PATH",
        format!(
            "Refusing {}: it contains a Git directory at {inside}",
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
