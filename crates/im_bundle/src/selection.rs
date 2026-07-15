// Path: crates/im_bundle/src/selection.rs
// Description: Canonical bundle-selection predicate shared by scanning and Git capture

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use crate::error::{BundleError, Result};
use crate::global_excludes::{
    is_globally_excluded_dir_name, is_globally_excluded_file_name, is_globally_excluded_path,
    normalize_global_excludes, NormalizedGlobalExcludes,
};
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
    excluded_subdirs: HashSet<String>,
    excluded_files: HashSet<String>,
    global_excludes: NormalizedGlobalExcludes,
}

impl BundleSelector {
    pub(crate) fn new(
        selection: &BundleSelection,
        global_excludes: &GlobalExcludes,
    ) -> Result<Self> {
        Ok(Self {
            include_root: selection.include_root,
            top_level_dirs: selection
                .top_level_dirs
                .iter()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect(),
            included_subdirs: normalize_selection_paths(
                &selection.included_subdirs,
                "includedSubdirs",
            )?
            .into_iter()
            .collect(),
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

    pub(crate) fn admits_git_path(&self, relative_path: &Path, kind: SelectedPathKind) -> bool {
        self.admits(relative_path, kind)
    }

    pub(crate) fn admits_directory(&self, relative_path: &Path) -> bool {
        self.admits(relative_path, SelectedPathKind::DirectoryLike)
    }

    fn admits(&self, relative_path: &Path, kind: SelectedPathKind) -> bool {
        if kind == SelectedPathKind::Symlink {
            return false;
        }
        let Some((components, archive_path)) = normalized_components(relative_path) else {
            return false;
        };
        if components.is_empty() {
            return false;
        }

        let selected = if components.len() == 1 {
            self.include_root || self.top_level_dirs.contains(&components[0])
        } else {
            self.top_level_dirs.contains(&components[0])
        };
        if !selected || self.excluded_files.contains(&archive_path) {
            return false;
        }

        let directory_count = if kind == SelectedPathKind::DirectoryLike {
            components.len()
        } else {
            components.len().saturating_sub(1)
        };
        for index in 0..directory_count {
            let directory_path = components[..=index].join("/");
            if self.excluded_subdirs.contains(&directory_path) {
                return false;
            }
            let explicitly_included = index == 0 || self.included_subdirs.contains(&directory_path);
            if !explicitly_included
                && is_globally_excluded_dir_name(&components[index], &self.global_excludes)
            {
                return false;
            }
        }

        if kind == SelectedPathKind::File
            && components
                .last()
                .is_some_and(|name| is_globally_excluded_file_name(name, &self.global_excludes))
        {
            return false;
        }

        !is_globally_excluded_path(&archive_path, &self.global_excludes)
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
mod tests {
    use super::*;

    fn selector() -> BundleSelector {
        BundleSelector::new(
            &BundleSelection {
                include_root: true,
                top_level_dirs: vec!["src".to_string()],
                included_subdirs: vec![],
                excluded_subdirs: vec!["src/private".to_string()],
                excluded_files: vec!["src/skip.rs".to_string()],
            },
            &GlobalExcludes {
                dir_names: vec!["target".to_string()],
                dir_suffixes: vec![],
                file_names: vec![],
                extensions: vec![".bin".to_string()],
                patterns: vec![],
            },
        )
        .expect("selector")
    }

    #[test]
    fn applies_one_predicate_to_current_and_missing_paths() {
        let selector = selector();
        assert!(selector.admits_file(Path::new("README.md")));
        assert!(selector.admits_file(Path::new("src/main.rs")));
        assert!(selector.admits_file(Path::new("src/deleted.rs")));
        assert!(!selector.admits_file(Path::new("src/private/key.rs")));
        assert!(!selector.admits_file(Path::new("src/skip.rs")));
        assert!(!selector.admits_file(Path::new("src/blob.bin")));
        assert!(!selector.admits_file(Path::new("other/file.rs")));
    }

    #[test]
    fn explicit_top_level_selection_overrides_directory_name_exclude() {
        let selector = selector();
        assert!(!selector.admits_file(Path::new("target/output.rs")));

        let selector = BundleSelector::new(
            &BundleSelection {
                include_root: false,
                top_level_dirs: vec!["target".to_string()],
                included_subdirs: vec![],
                excluded_subdirs: vec![],
                excluded_files: vec![],
            },
            &GlobalExcludes {
                dir_names: vec!["target".to_string()],
                dir_suffixes: vec![],
                file_names: vec![],
                extensions: vec![],
                patterns: vec![],
            },
        )
        .expect("selector");

        assert!(selector.admits_directory(Path::new("target")));
        assert!(selector.admits_file(Path::new("target/output.rs")));
    }

    #[test]
    fn explicit_nested_selection_overrides_only_its_directory_name_exclude() {
        let selector = BundleSelector::new(
            &BundleSelection {
                include_root: false,
                top_level_dirs: vec!["crates".to_string()],
                included_subdirs: vec!["crates/wb_render_wgpu/src/target".to_string()],
                excluded_subdirs: vec![],
                excluded_files: vec![],
            },
            &GlobalExcludes {
                dir_names: vec!["target".to_string(), "node_modules".to_string()],
                dir_suffixes: vec![],
                file_names: vec![],
                extensions: vec![],
                patterns: vec![],
            },
        )
        .expect("selector");

        assert!(selector.admits_file(Path::new("crates/wb_render_wgpu/src/target/mod.rs")));
        assert!(!selector.admits_file(Path::new(
            "crates/wb_render_wgpu/src/target/node_modules/noise.js"
        )));
        assert!(!selector.admits_file(Path::new("crates/other/target/output.rs")));
    }
}
