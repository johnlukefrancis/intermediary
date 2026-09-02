// Path: crates/im_bundle/src/git_capture/status.rs
// Description: Raw porcelain-v2 Git status parsing and selection-safe projection

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use crate::omission::OmissionReason;
use crate::selection::{BundleSelector, SelectedPathKind};

use super::path::{bytes_to_path, strip_repo_prefix, GitPath};
use super::porcelain::parse_porcelain;
use super::GitCaptureCounts;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectedRecordKind {
    Changed,
    Renamed,
    Unmerged,
    Untracked,
    IgnoredUntracked,
}

#[derive(Debug, Clone)]
pub(crate) struct SelectedStatusRecord {
    pub(crate) kind: SelectedRecordKind,
    pub(crate) xy: String,
    pub(crate) current: Option<GitPath>,
    pub(crate) original: Option<GitPath>,
    pub(crate) score: Option<String>,
    pub(crate) counterpart_omitted: bool,
}

/// A changed repository path the selection left out: name and reason cross
/// into evidence, content never does.
#[derive(Debug, Clone)]
pub(crate) struct OmittedPath {
    pub(crate) xy: String,
    pub(crate) path: GitPath,
    pub(crate) reason: OmissionReason,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedStatus {
    pub(crate) head_sha: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) repo_dirty: bool,
    pub(crate) selection_dirty: bool,
    pub(crate) counts: GitCaptureCounts,
    pub(crate) selected_records: Vec<SelectedStatusRecord>,
    pub(crate) omitted: Vec<OmittedPath>,
    pub(crate) general_pathspecs: Vec<GitPath>,
    pub(crate) rename_pathspecs: Vec<[GitPath; 2]>,
    pub(crate) watched_regular_paths: HashSet<PathBuf>,
}

pub(crate) fn parse_status(
    output: &[u8],
    repo_prefix: &[u8],
    repo_root: &Path,
    selector: &BundleSelector,
) -> std::result::Result<ParsedStatus, String> {
    let porcelain = parse_porcelain(output)?;
    let head_sha = porcelain.head_sha;
    let branch = porcelain.branch;
    let records = porcelain.records;

    let repo_dirty = !records.is_empty();
    let mut selected_paths = HashSet::new();
    let mut selected_tracked = HashSet::new();
    let mut selected_untracked = HashSet::new();
    let mut selected_deleted = HashSet::new();
    let mut omitted_paths = BTreeMap::new();
    let mut selected_records = Vec::new();
    let mut general_pathspecs = HashSet::new();
    let mut rename_pathspecs = HashSet::new();
    let mut watched_regular_paths = HashSet::new();
    let mut renamed_count = 0u64;
    let mut conflicted_count = 0u64;

    for record in records {
        let current = project_path(
            &record.current,
            repo_prefix,
            record.current_kind(repo_root, repo_prefix),
            selector,
        );
        let original = record
            .original
            .as_ref()
            .map(|path| project_path(path, repo_prefix, record.original_kind(), selector));

        register_endpoint(
            &record.current,
            &record.xy,
            &current,
            record.is_untracked(),
            record.is_deleted(),
            &mut selected_paths,
            &mut selected_tracked,
            &mut selected_untracked,
            &mut selected_deleted,
            &mut omitted_paths,
        );
        if let (Some(original_path), Some(projected)) = (&record.original, &original) {
            register_endpoint(
                original_path,
                &record.xy,
                projected,
                false,
                false,
                &mut selected_paths,
                &mut selected_tracked,
                &mut selected_untracked,
                &mut selected_deleted,
                &mut omitted_paths,
            );
        }

        let selected_current = current.clone().ok();
        let selected_original = original.clone().and_then(|projected| projected.ok());
        let counterpart_omitted =
            record.original.is_some() && (current.is_err() || matches!(original, Some(Err(_))));
        if selected_current.is_none() && selected_original.is_none() {
            continue;
        }

        if !record.is_untracked() {
            let fully_selected_rename = record.original.is_some()
                && selected_current.is_some()
                && selected_original.is_some();
            if fully_selected_rename {
                if let (Some(current), Some(original)) = (&selected_current, &selected_original) {
                    rename_pathspecs.insert([current.clone(), original.clone()]);
                }
            } else {
                if let Some(path) = &selected_current {
                    general_pathspecs.insert(path.clone());
                }
                if let Some(path) = &selected_original {
                    general_pathspecs.insert(path.clone());
                }
            }
        }
        if record.current_kind(repo_root, repo_prefix) == SelectedPathKind::File
            && !record.is_deleted()
        {
            if let Some(path) = &selected_current {
                if let Some(path_buf) = path.to_path_buf() {
                    watched_regular_paths.insert(path_buf);
                }
            }
        }

        let kind = if record.is_untracked() {
            SelectedRecordKind::Untracked
        } else if record.is_unmerged() {
            conflicted_count += 1;
            SelectedRecordKind::Unmerged
        } else if record.original.is_some() {
            if selected_current.is_some() && selected_original.is_some() {
                renamed_count += 1;
            }
            SelectedRecordKind::Renamed
        } else {
            SelectedRecordKind::Changed
        };
        selected_records.push(SelectedStatusRecord {
            kind,
            xy: record.xy,
            current: selected_current,
            original: selected_original,
            score: record.score,
            counterpart_omitted,
        });
    }

    let mut general_pathspecs: Vec<_> = general_pathspecs.into_iter().collect();
    general_pathspecs.sort();
    let mut rename_pathspecs: Vec<_> = rename_pathspecs.into_iter().collect();
    rename_pathspecs.sort();
    selected_records.sort_by(|left, right| {
        let left_path = left.current.as_ref().or(left.original.as_ref());
        let right_path = right.current.as_ref().or(right.original.as_ref());
        left_path.cmp(&right_path)
    });

    Ok(ParsedStatus {
        head_sha,
        branch,
        repo_dirty,
        selection_dirty: !selected_paths.is_empty(),
        counts: GitCaptureCounts {
            selected_changed: selected_paths.len() as u64,
            selected_tracked_changed: selected_tracked.len() as u64,
            selected_untracked: selected_untracked.len() as u64,
            selected_deleted: selected_deleted.len() as u64,
            selected_renamed: renamed_count,
            selected_conflicted: conflicted_count,
            omitted_changed_paths: Some(omitted_paths.len() as u64),
        },
        selected_records,
        omitted: omitted_paths.into_values().collect(),
        general_pathspecs,
        rename_pathspecs,
        watched_regular_paths,
    })
}

impl ParsedStatus {
    pub(crate) fn add_ignored_untracked(&mut self, ignored_paths: &[GitPath]) {
        let mut existing: HashSet<GitPath> = self
            .selected_records
            .iter()
            .filter_map(|record| record.current.clone())
            .collect();
        for path in ignored_paths {
            if !existing.insert(path.clone()) {
                continue;
            }
            if let Some(path_buf) = path.to_path_buf() {
                self.watched_regular_paths.insert(path_buf);
            }
            self.selected_records.push(SelectedStatusRecord {
                kind: SelectedRecordKind::IgnoredUntracked,
                xy: "!!".to_string(),
                current: Some(path.clone()),
                original: None,
                score: None,
                counterpart_omitted: false,
            });
            self.counts.selected_changed = self.counts.selected_changed.saturating_add(1);
            self.counts.selected_untracked = self.counts.selected_untracked.saturating_add(1);
        }
        self.selection_dirty = self.counts.selected_changed > 0;
        self.selected_records.sort_by(|left, right| {
            let left_path = left.current.as_ref().or(left.original.as_ref());
            let right_path = right.current.as_ref().or(right.original.as_ref());
            left_path.cmp(&right_path)
        });
    }
}

fn project_path(
    path: &GitPath,
    repo_prefix: &[u8],
    kind: SelectedPathKind,
    selector: &BundleSelector,
) -> std::result::Result<GitPath, OmissionReason> {
    let relative =
        strip_repo_prefix(path.as_bytes(), repo_prefix).ok_or(OmissionReason::OutsideBundleRoot)?;
    let path_buf = bytes_to_path(relative).ok_or(OmissionReason::UnrepresentablePath)?;
    selector.classify(&path_buf, kind)?;
    Ok(GitPath::from_bytes(relative))
}

#[allow(clippy::too_many_arguments)]
fn register_endpoint(
    repo_path: &GitPath,
    xy: &str,
    selected: &std::result::Result<GitPath, OmissionReason>,
    untracked: bool,
    deleted: bool,
    selected_paths: &mut HashSet<GitPath>,
    selected_tracked: &mut HashSet<GitPath>,
    selected_untracked: &mut HashSet<GitPath>,
    selected_deleted: &mut HashSet<GitPath>,
    omitted_paths: &mut BTreeMap<GitPath, OmittedPath>,
) {
    let path = match selected {
        Ok(path) => path,
        Err(reason) => {
            omitted_paths
                .entry(repo_path.clone())
                .or_insert_with(|| OmittedPath {
                    xy: xy.to_string(),
                    path: repo_path.clone(),
                    reason: reason.clone(),
                });
            return;
        }
    };
    selected_paths.insert(path.clone());
    if untracked {
        selected_untracked.insert(path.clone());
    } else {
        selected_tracked.insert(path.clone());
    }
    if deleted {
        selected_deleted.insert(path.clone());
    }
}
