// Path: crates/im_bundle/src/selection.rs
// Description: Canonical bundle-selection predicate shared by scanning and Git capture

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use crate::error::{BundleError, Result};
use crate::global_excludes::{
    is_globally_excluded_dir_name, is_globally_excluded_file_name, is_globally_excluded_path,
    normalize_global_excludes, NormalizedGlobalExcludes,
};
use crate::omission::OmissionReason;
use crate::plan::{BundleSelection, GlobalExcludes};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectedPathKind {
    File,
    DirectoryLike,
    Symlink,
}

#[derive(Debug, Clone)]
pub(crate) struct BundleSelector {
    include_root: bool,
    top_level_dirs: HashSet<String>,
    included_subdirs: HashSet<String>,
    included_files: HashSet<String>,
    traversal_ancestors: HashSet<String>,
    excluded_subdirs: HashSet<String>,
    excluded_files: HashSet<String>,
    global_excludes: NormalizedGlobalExcludes,
}

impl BundleSelector {
    pub(crate) fn new(
        selection: &BundleSelection,
        global_excludes: &GlobalExcludes,
    ) -> Result<Self> {
        let included_subdirs: HashSet<_> =
            normalize_selection_paths(&selection.included_subdirs, "includedSubdirs")?
                .into_iter()
                .collect();
        let included_files: HashSet<_> =
            normalize_selection_paths(&selection.included_files, "includedFiles")?
                .into_iter()
                .collect();
        let mut traversal_ancestors = HashSet::new();
        for path in included_subdirs.iter().chain(included_files.iter()) {
            let mut parent = path.as_str();
            while let Some((ancestor, _)) = parent.rsplit_once('/') {
                traversal_ancestors.insert(ancestor.to_string());
                parent = ancestor;
            }
        }
        Ok(Self {
            include_root: selection.include_root,
            top_level_dirs: selection
                .top_level_dirs
                .iter()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect(),
            included_subdirs,
            included_files,
            traversal_ancestors,
            excluded_subdirs: normalize_excluded_paths(
                &selection.excluded_subdirs,
                "excludedSubdirs",
            )?
            .into_iter()
            .collect(),
            excluded_files: normalize_excluded_paths(&selection.excluded_files, "excludedFiles")?
                .into_iter()
                .collect(),
            global_excludes: normalize_global_excludes(global_excludes),
        })
    }

    pub(crate) fn admits_file(&self, relative_path: &Path) -> bool {
        self.admits(relative_path, SelectedPathKind::File)
    }

    pub(crate) fn admits_directory(&self, relative_path: &Path) -> bool {
        self.admits(relative_path, SelectedPathKind::DirectoryLike)
    }

    pub(crate) fn traverses_directory(&self, path: &Path) -> bool {
        let Some((_, archive_path)) = normalized_components(path) else {
            return false;
        };
        if is_globally_excluded_path(&archive_path, &self.global_excludes) {
            return false;
        }
        self.admits_directory(path) || self.traversal_ancestors.contains(&archive_path)
    }

    pub(crate) fn top_level_dirs(&self) -> Vec<String> {
        let mut roots = self.top_level_dirs.clone();
        for path in &self.included_subdirs {
            roots.insert(path.split('/').next().unwrap_or(path).to_string());
        }
        for path in &self.included_files {
            if let Some((root, _)) = path.split_once('/') {
                roots.insert(root.to_string());
            }
        }
        roots.into_iter().collect()
    }

    pub(crate) fn includes_root_files(&self) -> bool {
        self.include_root || self.included_files.iter().any(|path| !path.contains('/'))
    }

    fn admits(&self, relative_path: &Path, kind: SelectedPathKind) -> bool {
        self.classify(relative_path, kind).is_ok()
    }

    /// The one selection predicate, answering why a path is omitted when it is.
    pub(crate) fn classify(
        &self,
        relative_path: &Path,
        kind: SelectedPathKind,
    ) -> std::result::Result<(), OmissionReason> {
        if kind == SelectedPathKind::Symlink {
            return Err(OmissionReason::Symlink);
        }
        let Some((components, archive_path)) = normalized_components(relative_path) else {
            return Err(OmissionReason::UnrepresentablePath);
        };
        let Some(top) = components.first() else {
            return Err(OmissionReason::UnrepresentablePath);
        };

        let mut omission = if components.len() == 1 && kind == SelectedPathKind::File {
            (!self.include_root).then_some(OmissionReason::RootFilesNotSelected)
        } else {
            Some(OmissionReason::TopLevelDirNotSelected(top.clone()))
        };
        let directory_count = if kind == SelectedPathKind::DirectoryLike {
            components.len()
        } else {
            components.len().saturating_sub(1)
        };
        for index in 0..directory_count {
            let directory_path = components[..=index].join("/");
            if self.excluded_subdirs.contains(&directory_path) {
                omission = Some(OmissionReason::ExcludedSubdir(directory_path));
            } else if self.included_subdirs.contains(&directory_path)
                || (index == 0 && self.top_level_dirs.contains(top))
            {
                omission = None;
            } else if is_globally_excluded_dir_name(&components[index], &self.global_excludes) {
                omission = Some(OmissionReason::GlobalDirName(components[index].clone()));
            }
        }
        if kind == SelectedPathKind::File && self.included_files.contains(&archive_path) {
            omission = None;
        }
        if self.excluded_files.contains(&archive_path) {
            return Err(OmissionReason::ExcludedFile);
        }

        if kind == SelectedPathKind::File {
            if let Some(name) = components.last() {
                if is_globally_excluded_file_name(name, &self.global_excludes) {
                    return Err(OmissionReason::GlobalFileName(name.clone()));
                }
            }
        }

        if is_globally_excluded_path(&archive_path, &self.global_excludes) {
            return Err(OmissionReason::GlobalPathPattern);
        }
        omission.map_or(Ok(()), Err)
    }
}

fn normalized_components(path: &Path) -> Option<(Vec<String>, String)> {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => components.push(value.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::Prefix(_) | Component::RootDir | Component::ParentDir => return None,
        }
    }
    let archive_path = components.join("/");
    Some((components, archive_path))
}

fn normalize_selection_paths(excluded: &[String], field_name: &str) -> Result<Vec<String>> {
    let mut normalized = Vec::new();
    for item in excluded {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        let normalized_item = trimmed.replace('\\', "/");
        let path = PathBuf::from(&normalized_item);
        if normalized_item.starts_with('/')
            || path.components().any(|component| {
                matches!(
                    component,
                    Component::Prefix(_) | Component::RootDir | Component::ParentDir
                )
            })
        {
            return Err(BundleError::InvalidPlan(format!(
                "{field_name} must be a repo-relative path without '..': {trimmed}"
            )));
        }
        normalized.push(normalized_item);
    }
    Ok(normalized)
}

fn normalize_excluded_paths(excluded: &[String], field_name: &str) -> Result<Vec<String>> {
    normalize_selection_paths(excluded, field_name)
}

#[cfg(test)]
#[path = "selection_tests.rs"]
mod tests;
