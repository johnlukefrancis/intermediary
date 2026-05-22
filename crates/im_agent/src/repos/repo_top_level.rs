// Path: crates/im_agent/src/repos/repo_top_level.rs
// Description: Scan top-level entries and bounded nested bundle-selector directory paths

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use tokio::fs;

use crate::bundles::ignore_rules::{default_excluded_dir_names, should_ignore_entry};

const BUNDLE_SELECTOR_MAX_REPO_DEPTH: usize = 4;
const TOP_LEVEL_DEPTH: usize = 1;
const MAX_SUBDIR_DEPTH: usize = BUNDLE_SELECTOR_MAX_REPO_DEPTH - TOP_LEVEL_DEPTH;

#[derive(Debug, Clone)]
pub struct TopLevelResult {
    pub dirs: Vec<String>,
    pub files: Vec<String>,
    pub subdirs: HashMap<String, Vec<String>>,
    pub default_excluded: Vec<String>,
}

pub async fn get_repo_top_level(root_path: &str) -> Result<TopLevelResult, std::io::Error> {
    let root = Path::new(root_path);
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    let mut subdirs = HashMap::new();
    let excluded_names = default_excluded_dir_names();

    let mut entries = fs::read_dir(root).await?;
    while let Some(entry) = entries.next_entry().await? {
        let file_type = entry.file_type().await?;
        let name = entry.file_name().to_string_lossy().to_string();
        if file_type.is_dir() {
            dirs.push(name);
        } else if file_type.is_file() {
            if !should_ignore_entry(&name, false) {
                files.push(name);
            }
        }
    }

    dirs.sort();
    files.sort();

    for dir in &dirs {
        // Skip subdir scanning for default-excluded dirs (performance guard)
        if excluded_names.iter().any(|e| e == dir) {
            subdirs.insert(dir.clone(), Vec::new());
            continue;
        }

        let mut names = collect_subdir_paths(&root.join(dir)).await;
        names.sort();
        subdirs.insert(dir.clone(), names);
    }

    let default_excluded = excluded_names.iter().map(|s| (*s).to_string()).collect();

    Ok(TopLevelResult {
        dirs,
        files,
        subdirs,
        default_excluded,
    })
}

pub async fn is_valid_repo_root(root_path: &str) -> bool {
    match fs::metadata(root_path).await {
        Ok(metadata) => metadata.is_dir(),
        Err(_) => false,
    }
}

async fn collect_subdir_paths(dir_path: &Path) -> Vec<String> {
    let mut names = Vec::new();
    let mut pending = VecDeque::new();
    collect_child_dirs(dir_path, "", 1, &mut pending, &mut names).await;

    while let Some((path, relative_path, depth)) = pending.pop_front() {
        if depth >= MAX_SUBDIR_DEPTH {
            continue;
        }
        collect_child_dirs(&path, &relative_path, depth + 1, &mut pending, &mut names).await;
    }

    names
}

async fn collect_child_dirs(
    dir_path: &Path,
    relative_root: &str,
    child_depth: usize,
    pending: &mut VecDeque<(PathBuf, String, usize)>,
    names: &mut Vec<String>,
) {
    let Ok(mut entries) = fs::read_dir(dir_path).await else {
        return;
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        let Ok(file_type) = entry.file_type().await else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        let relative_path = if relative_root.is_empty() {
            name.clone()
        } else {
            format!("{relative_root}/{name}")
        };
        names.push(relative_path.clone());

        if child_depth < MAX_SUBDIR_DEPTH && !should_ignore_entry(&name, true) {
            pending.push_back((entry.path(), relative_path, child_depth));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::get_repo_top_level;
    use tempfile::tempdir;

    #[tokio::test]
    async fn repo_top_level_lists_subdir_paths_to_depth_four() {
        let dir = tempdir().expect("create temp repo");
        let repo_root = dir.path();

        tokio::fs::create_dir_all(repo_root.join("app/src/components/bundles/deeper"))
            .await
            .expect("create nested dirs");
        tokio::fs::create_dir_all(repo_root.join("app/src/node_modules/pkg"))
            .await
            .expect("create ignored nested dirs");

        let result = get_repo_top_level(repo_root.to_str().expect("temp path is utf8"))
            .await
            .expect("scan repo");

        assert_eq!(result.dirs, vec!["app".to_string()]);
        assert_eq!(
            result.subdirs.get("app").expect("app subdirs"),
            &vec![
                "src".to_string(),
                "src/components".to_string(),
                "src/components/bundles".to_string(),
                "src/node_modules".to_string(),
            ]
        );
    }
}
