// Path: crates/im_bundle/src/scanner.rs
// Description: Bundle scanning logic with ignore rules and exclusions

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::cancel::{check_cancelled, BundleCancelToken};
use crate::error::{BundleError, Result};
use crate::plan::BundlePlan;
use crate::progress::ProgressEmitter;
use crate::selection::BundleSelector;

#[derive(Debug, Clone)]
pub struct ScanEntry {
    pub source_path: PathBuf,
    pub repo_relative_path: PathBuf,
    pub archive_path: String,
}

#[derive(Debug)]
pub struct ScanResult {
    pub entries: Vec<ScanEntry>,
    pub top_level_dirs_included: Vec<String>,
}

pub fn scan_bundle(plan: &BundlePlan, progress: &mut ProgressEmitter) -> Result<ScanResult> {
    scan_bundle_with_cancel(plan, progress, None)
}

pub fn scan_bundle_with_cancel(
    plan: &BundlePlan,
    progress: &mut ProgressEmitter,
    cancel_token: Option<&BundleCancelToken>,
) -> Result<ScanResult> {
    let repo_root = &plan.repo_root;
    if !repo_root.exists() {
        return Err(BundleError::RepoRootMissing {
            path: repo_root.clone(),
        });
    }
    check_cancelled(cancel_token)?;

    let selector = BundleSelector::new(&plan.selection, &plan.global_excludes)?;

    let mut entries = Vec::new();
    let mut files_scanned = 0u64;

    if plan.selection.include_root {
        let root_entries =
            std::fs::read_dir(repo_root).map_err(|source| BundleError::DirReadFailed {
                path: repo_root.clone(),
                source,
            })?;
        for entry in root_entries {
            check_cancelled(cancel_token)?;
            let entry = entry.map_err(|source| BundleError::DirReadFailed {
                path: repo_root.clone(),
                source,
            })?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            let file_type = entry
                .file_type()
                .map_err(|source| BundleError::MetadataFailed {
                    path: entry.path(),
                    source,
                })?;
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_file() {
                let relative_path = PathBuf::from(&name);
                if !selector.admits_file(&relative_path) {
                    continue;
                }
                entries.push(ScanEntry {
                    source_path: entry.path(),
                    repo_relative_path: relative_path,
                    archive_path: name_str.to_string(),
                });
                files_scanned += 1;
                progress.emit_progress("scanning", files_scanned, 0);
            }
        }
    }

    let top_level_dirs = validate_top_level_dirs(repo_root, &plan.selection.top_level_dirs)?;
    let mut top_level_dirs_included = Vec::new();
    for dir in &top_level_dirs {
        check_cancelled(cancel_token)?;
        let relative_dir = PathBuf::from(dir);
        if !selector.admits_directory(&relative_dir) {
            continue;
        }
        top_level_dirs_included.push(dir.to_string());
        let dir_path = repo_root.join(dir);
        collect_dir_entries(
            &mut entries,
            &dir_path,
            &relative_dir,
            &selector,
            &mut files_scanned,
            progress,
            cancel_token,
        )?;
    }

    entries.sort_by(|left, right| left.archive_path.cmp(&right.archive_path));

    Ok(ScanResult {
        entries,
        top_level_dirs_included,
    })
}

fn validate_top_level_dirs(repo_root: &Path, dirs: &[String]) -> Result<Vec<String>> {
    let mut unique = HashSet::new();
    let mut included = Vec::new();

    for dir in dirs {
        let trimmed = dir.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "." || trimmed == ".." || trimmed.contains('/') || trimmed.contains('\\') {
            return Err(BundleError::InvalidPlan(format!(
                "topLevelDirs must be simple directory names: {trimmed}"
            )));
        }
        if trimmed.split('/').any(|part| part == "..") {
            return Err(BundleError::InvalidPlan(format!(
                "topLevelDirs cannot contain '..': {trimmed}"
            )));
        }
        if !unique.insert(trimmed.to_string()) {
            continue;
        }
        let dir_path = repo_root.join(trimmed);
        if !dir_path.exists() {
            return Err(BundleError::TopLevelDirMissing {
                dir: trimmed.to_string(),
            });
        }
        if !dir_path.is_dir() {
            return Err(BundleError::TopLevelDirNotDirectory {
                dir: trimmed.to_string(),
            });
        }
        included.push(trimmed.to_string());
    }

    included.sort();
    Ok(included)
}

fn collect_dir_entries(
    entries: &mut Vec<ScanEntry>,
    dir_path: &Path,
    relative_dir: &Path,
    selector: &BundleSelector,
    files_scanned: &mut u64,
    progress: &mut ProgressEmitter,
    cancel_token: Option<&BundleCancelToken>,
) -> Result<()> {
    check_cancelled(cancel_token)?;

    let dir_entries = std::fs::read_dir(dir_path).map_err(|source| BundleError::DirReadFailed {
        path: dir_path.to_path_buf(),
        source,
    })?;

    for entry in dir_entries {
        check_cancelled(cancel_token)?;
        let entry = entry.map_err(|source| BundleError::DirReadFailed {
            path: dir_path.to_path_buf(),
            source,
        })?;
        let name = entry.file_name();
        let file_type = entry
            .file_type()
            .map_err(|source| BundleError::MetadataFailed {
                path: entry.path(),
                source,
            })?;

        if file_type.is_symlink() {
            continue;
        }

        let next_relative = relative_dir.join(&name);
        let next_archive = archive_path(&next_relative);
        if file_type.is_dir() {
            if !selector.admits_directory(&next_relative) {
                continue;
            }
            collect_dir_entries(
                entries,
                &entry.path(),
                &next_relative,
                selector,
                files_scanned,
                progress,
                cancel_token,
            )?;
            continue;
        }

        if file_type.is_file() {
            if !selector.admits_file(&next_relative) {
                continue;
            }
            entries.push(ScanEntry {
                source_path: entry.path(),
                repo_relative_path: next_relative,
                archive_path: next_archive,
            });
            *files_scanned += 1;
            progress.emit_progress("scanning", *files_scanned, 0);
        }
    }

    Ok(())
}

fn archive_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_string_lossy()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}
