// Path: crates/im_agent/src/repos/repo_top_level.rs
// Description: Scan top-level directories and files in a repo

use std::collections::HashMap;
use std::path::Path;

use tokio::fs;

use crate::bundles::ignore_rules::{default_excluded_dir_names, should_ignore_entry};

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

        let dir_path = root.join(dir);
        let mut names = Vec::new();
        let read_result = fs::read_dir(&dir_path).await;
        if let Ok(mut sub_entries) = read_result {
            while let Ok(Some(sub_entry)) = sub_entries.next_entry().await {
                let file_type = match sub_entry.file_type().await {
                    Ok(file_type) => file_type,
                    Err(_) => continue,
                };
                if !file_type.is_dir() {
                    continue;
                }
                let name = sub_entry.file_name().to_string_lossy().to_string();
                names.push(name);
            }
        }
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
