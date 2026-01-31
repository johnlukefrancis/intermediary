// Path: crates/im_bundle/src/scanner.rs
// Description: Bundle scanning logic with ignore rules and exclusions

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::error::{BundleError, Result};
use crate::ignore_rules::{is_ignored_dir, is_ignored_file};
use crate::plan::BundlePlan;
use crate::progress::ProgressEmitter;

#[derive(Debug, Clone)]
pub struct ScanEntry {
    pub source_path: PathBuf,
    pub archive_path: String,
}

#[derive(Debug)]
pub struct ScanResult {
    pub entries: Vec<ScanEntry>,
    pub top_level_dirs_included: Vec<String>,
}

pub fn scan_bundle(plan: &BundlePlan, progress: &mut ProgressEmitter) -> Result<ScanResult> {
    let repo_root = &plan.repo_root;
    if !repo_root.exists() {
        return Err(BundleError::RepoRootMissing {
            path: repo_root.clone(),
        });
    }

    let excluded = normalize_excluded(&plan.selection.excluded_subdirs)?;
    let excluded_set: HashSet<String> = excluded.into_iter().collect();

    let mut entries = Vec::new();
    let mut files_scanned = 0u64;

    if plan.selection.include_root {
        let root_entries = std::fs::read_dir(repo_root).map_err(|source| {
            BundleError::DirReadFailed {
                path: repo_root.clone(),
                source,
            }
        })?;
        for entry in root_entries {
            let entry = entry.map_err(|source| BundleError::DirReadFailed {
                path: repo_root.clone(),
                source,
            })?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            let file_type = entry.file_type().map_err(|source| BundleError::MetadataFailed {
                path: entry.path(),
                source,
            })?;
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_file() {
                if is_ignored_file(&name_str) {
                    continue;
                }
                let archive_path = name_str.to_string();
                entries.push(ScanEntry {
                    source_path: entry.path(),
                    archive_path,
                });
                files_scanned += 1;
                progress.emit_progress("scanning", files_scanned, 0);
            }
        }
    }

    let top_level_dirs = validate_top_level_dirs(repo_root, &plan.selection.top_level_dirs)?;
    for dir in &top_level_dirs {
        let dir_path = repo_root.join(dir);
        collect_dir_entries(
            &mut entries,
            &dir_path,
            dir,
            &excluded_set,
            &mut files_scanned,
            progress,
        )?;
    }

    Ok(ScanResult {
        entries,
        top_level_dirs_included: top_level_dirs,
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
        if is_ignored_dir(trimmed) {
            return Err(BundleError::TopLevelDirIgnored {
                dir: trimmed.to_string(),
            });
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
    archive_root: &str,
    excluded_set: &HashSet<String>,
    files_scanned: &mut u64,
    progress: &mut ProgressEmitter,
) -> Result<()> {
    if excluded_set.contains(archive_root) {
        return Ok(());
    }

    let dir_entries = std::fs::read_dir(dir_path).map_err(|source| BundleError::DirReadFailed {
        path: dir_path.to_path_buf(),
        source,
    })?;

    for entry in dir_entries {
        let entry = entry.map_err(|source| BundleError::DirReadFailed {
            path: dir_path.to_path_buf(),
            source,
        })?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        let file_type = entry.file_type().map_err(|source| BundleError::MetadataFailed {
            path: entry.path(),
            source,
        })?;

        if file_type.is_symlink() {
            continue;
        }

        let next_archive = join_archive(archive_root, &name_str);
        if file_type.is_dir() {
            if is_ignored_dir(&name_str) {
                continue;
            }
            collect_dir_entries(
                entries,
                &entry.path(),
                &next_archive,
                excluded_set,
                files_scanned,
                progress,
            )?;
            continue;
        }

        if file_type.is_file() {
            if is_ignored_file(&name_str) {
                continue;
            }
            entries.push(ScanEntry {
                source_path: entry.path(),
                archive_path: next_archive,
            });
            *files_scanned += 1;
            progress.emit_progress("scanning", *files_scanned, 0);
        }
    }

    Ok(())
}

fn join_archive(root: &str, name: &str) -> String {
    if root.is_empty() {
        name.to_string()
    } else {
        format!("{root}/{name}")
    }
}

fn normalize_excluded(excluded: &[String]) -> Result<Vec<String>> {
    let mut normalized = Vec::new();
    for item in excluded {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        let normalized_item = trimmed.replace('\\', "/");
        if normalized_item.starts_with('/') {
            return Err(BundleError::InvalidPlan(format!(
                "excludedSubdirs must be relative: {trimmed}"
            )));
        }
        if normalized_item.split('/').any(|part| part == "..") {
            return Err(BundleError::InvalidPlan(format!(
                "excludedSubdirs cannot contain '..': {trimmed}"
            )));
        }
        normalized.push(normalized_item);
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BundleGitInfo, BundleSelection};
    use tempfile::tempdir;

    #[test]
    fn scan_respects_ignore_and_exclude() {
        let dir = tempdir().unwrap();
        let repo_root = dir.path();

        std::fs::create_dir(repo_root.join("app")).unwrap();
        std::fs::create_dir(repo_root.join("node_modules")).unwrap();
        std::fs::create_dir_all(repo_root.join("app/skip_me")).unwrap();

        std::fs::write(repo_root.join("README.md"), "root").unwrap();
        std::fs::write(repo_root.join(".env"), "secret").unwrap();
        std::fs::write(repo_root.join("app/index.ts"), "code").unwrap();
        std::fs::write(repo_root.join("app/skip_me/secret.txt"), "skip").unwrap();

        let plan = BundlePlan {
            output_path: repo_root.join("out.zip"),
            repo_root: repo_root.to_path_buf(),
            repo_id: "repo".to_string(),
            preset_id: "full".to_string(),
            preset_name: "Full".to_string(),
            selection: BundleSelection {
                include_root: true,
                top_level_dirs: vec!["app".to_string()],
                excluded_subdirs: vec!["app/skip_me".to_string()],
            },
            git: BundleGitInfo {
                head_sha: None,
                short_sha: None,
                branch: None,
            },
            built_at_iso: "2026-01-31T00:00:00Z".to_string(),
        };

        let mut progress = ProgressEmitter::new();
        let result = scan_bundle(&plan, &mut progress).unwrap();

        let archive_paths: HashSet<_> = result
            .entries
            .iter()
            .map(|entry| entry.archive_path.as_str())
            .collect();

        assert!(archive_paths.contains("README.md"));
        assert!(archive_paths.contains("app/index.ts"));
        assert!(!archive_paths.contains(".env"));
        assert!(!archive_paths.contains("app/skip_me/secret.txt"));
    }

    #[test]
    fn reject_invalid_top_level_dir() {
        let dir = tempdir().unwrap();
        let repo_root = dir.path();

        let plan = BundlePlan {
            output_path: repo_root.join("out.zip"),
            repo_root: repo_root.to_path_buf(),
            repo_id: "repo".to_string(),
            preset_id: "full".to_string(),
            preset_name: "Full".to_string(),
            selection: BundleSelection {
                include_root: false,
                top_level_dirs: vec!["../escape".to_string()],
                excluded_subdirs: vec![],
            },
            git: BundleGitInfo {
                head_sha: None,
                short_sha: None,
                branch: None,
            },
            built_at_iso: "2026-01-31T00:00:00Z".to_string(),
        };

        let mut progress = ProgressEmitter::new();
        let result = scan_bundle(&plan, &mut progress);
        assert!(matches!(result, Err(BundleError::InvalidPlan(_))));
    }
}
