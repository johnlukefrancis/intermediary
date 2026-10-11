// Path: crates/im_bundle/src/inspection.rs
// Description: Bounded metadata inspection using the archive's canonical selection predicate

use std::fs::{self, Metadata};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::cancel::{check_cancelled, BundleCancelToken};
use crate::error::{BundleError, Result};
use crate::plan::{BundleSelection, GlobalExcludes};
use crate::selection::BundleSelector;

const MAX_ENTRIES: usize = 200_000;
const MAX_DURATION: Duration = Duration::from_secs(3);
const LARGEST_FILES: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectedFile {
    pub path: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryInspection {
    pub is_directory: bool,
    pub bytes: u64,
    pub file_count: u64,
    pub included_bytes: u64,
    pub included_file_count: u64,
    pub largest_included_files: Vec<InspectedFile>,
    pub modified_ms: Option<u64>,
    pub complete: bool,
    pub unreadable_entries: u64,
}

pub fn inspect_entry(
    repo_root: &Path,
    relative_path: &str,
    selection: &BundleSelection,
    excludes: &GlobalExcludes,
    cancel: &BundleCancelToken,
) -> Result<EntryInspection> {
    inspect_with_limits(
        repo_root,
        relative_path,
        selection,
        excludes,
        cancel,
        MAX_ENTRIES,
        MAX_DURATION,
    )
}

fn inspect_with_limits(
    repo_root: &Path,
    relative_path: &str,
    selection: &BundleSelection,
    excludes: &GlobalExcludes,
    cancel: &BundleCancelToken,
    max_entries: usize,
    max_duration: Duration,
) -> Result<EntryInspection> {
    check_cancelled(Some(cancel))?;
    let start = Instant::now();
    let root = fs::canonicalize(repo_root).map_err(|source| BundleError::MetadataFailed {
        path: repo_root.to_path_buf(),
        source,
    })?;
    let relative = Path::new(relative_path);
    if relative_path.is_empty()
        || relative_path.contains('\\')
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(BundleError::InvalidPlan(
            "Inspection requires a repo-relative path".into(),
        ));
    }
    let mut target = root.clone();
    for part in relative.components() {
        if part
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(".git")
        {
            return Err(BundleError::InvalidPlan(
                "Git metadata cannot be inspected".into(),
            ));
        }
        target.push(part);
        let metadata =
            fs::symlink_metadata(&target).map_err(|source| BundleError::MetadataFailed {
                path: target.clone(),
                source,
            })?;
        if metadata.is_symlink() {
            return Err(BundleError::InvalidPlan(
                "Symbolic links are not included in bundles".into(),
            ));
        }
    }
    let target = contained_path(&root, &target)?;
    let metadata = fs::symlink_metadata(&target).map_err(|source| BundleError::MetadataFailed {
        path: target.clone(),
        source,
    })?;
    let selector = BundleSelector::new(selection, excludes)?;
    let mut result = EntryInspection {
        is_directory: metadata.is_dir(),
        bytes: 0,
        file_count: 0,
        included_bytes: 0,
        included_file_count: 0,
        largest_included_files: Vec::new(),
        complete: true,
        unreadable_entries: 0,
        modified_ms: metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as u64),
    };
    if metadata.is_file() {
        result.add_file(relative, &metadata, &selector);
        return Ok(result);
    }
    if !metadata.is_dir() {
        return Err(BundleError::InvalidPlan(
            "Only regular files and folders can be inspected".into(),
        ));
    }
    let entries = fs::read_dir(&target).map_err(|source| BundleError::DirReadFailed {
        path: target,
        source,
    })?;
    let mut stack = vec![(relative.to_path_buf(), entries)];
    let mut visited = 0;
    while let Some((parent, entries)) = stack.last_mut() {
        check_cancelled(Some(cancel))?;
        if visited >= max_entries || start.elapsed() >= max_duration {
            result.complete = false;
            break;
        }
        let Some(entry) = entries.next() else {
            stack.pop();
            continue;
        };
        visited += 1;
        let Ok(entry) = entry else {
            result.unreadable();
            continue;
        };
        if entry
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case(".git")
        {
            continue;
        }
        let path = parent.join(entry.file_name());
        let Ok(metadata) = fs::symlink_metadata(entry.path()) else {
            result.unreadable();
            continue;
        };
        if metadata.is_symlink() {
            continue;
        }
        if metadata.is_file() {
            result.add_file(&path, &metadata, &selector);
        } else if metadata.is_dir() {
            match contained_path(&root, &entry.path()).and_then(|dir| {
                fs::read_dir(&dir)
                    .map_err(|source| BundleError::DirReadFailed { path: dir, source })
            }) {
                Ok(children) => stack.push((path, children)),
                Err(_) => result.unreadable(),
            }
        }
    }
    check_cancelled(Some(cancel))?;
    Ok(result)
}

fn contained_path(root: &Path, path: &Path) -> Result<PathBuf> {
    let resolved = fs::canonicalize(path).map_err(|source| BundleError::MetadataFailed {
        path: path.to_path_buf(),
        source,
    })?;
    if !resolved.starts_with(root) {
        return Err(BundleError::InvalidPlan(
            "Inspection path escapes the repository".into(),
        ));
    }
    Ok(resolved)
}

impl EntryInspection {
    fn unreadable(&mut self) {
        self.unreadable_entries += 1;
        self.complete = false;
    }

    fn add_file(&mut self, path: &Path, metadata: &Metadata, selector: &BundleSelector) {
        self.bytes = self.bytes.saturating_add(metadata.len());
        self.file_count += 1;
        if !selector.admits_file(path) {
            return;
        }
        self.included_bytes = self.included_bytes.saturating_add(metadata.len());
        self.included_file_count += 1;
        let path = path
            .components()
            .map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        self.largest_included_files.push(InspectedFile {
            path,
            bytes: metadata.len(),
        });
        self.largest_included_files.sort_by(|left, right| {
            right
                .bytes
                .cmp(&left.bytes)
                .then_with(|| left.path.cmp(&right.path))
        });
        self.largest_included_files.truncate(LARGEST_FILES);
    }
}

#[cfg(test)]
#[path = "inspection_tests.rs"]
mod tests;
