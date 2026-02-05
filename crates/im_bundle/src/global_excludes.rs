// Path: crates/im_bundle/src/global_excludes.rs
// Description: Normalize and apply user-configurable global excludes for bundle scanning

use crate::plan::GlobalExcludes;

const RECOMMENDED_DIR_NAMES: &[&str] = &[
    ".cache",
    ".git",
    ".mypy_cache",
    ".next",
    ".nyc_output",
    ".nuxt",
    ".parcel-cache",
    ".pytest_cache",
    ".ruff_cache",
    ".svelte-kit",
    ".tox",
    ".turbo",
    ".venv",
    ".gradle",
    ".hypothesis",
    ".nox",
    "__pycache__",
    "build",
    "coverage",
    "dist",
    "env",
    "logs",
    "node_modules",
    "out",
    "target",
    "venv",
];

const RECOMMENDED_DIR_SUFFIXES: &[&str] = &[".egg-info"];

const RECOMMENDED_FILE_NAMES: &[&str] = &[
    ".ds_store",
    ".coverage",
    ".env",
    ".env.local",
    ".eslintcache",
    "thumbs.db",
];

const RECOMMENDED_FILE_SUFFIXES: &[&str] = &[
    ".bak",
    ".log",
    ".old",
    ".orig",
    ".pyc",
    ".pyd",
    ".pyo",
    ".swo",
    ".swp",
    ".tmp",
    "~",
    ".gguf",
    ".safetensors",
    ".ckpt",
    ".pt",
    ".pth",
    ".bin",
    ".onnx",
    ".pb",
    ".h5",
    ".keras",
    ".exe",
    ".dll",
    ".so",
    ".dylib",
    ".pdb",
    ".lib",
    ".a",
];

const RECOMMENDED_PATH_SEGMENTS: &[&str] = &[
    "models",
    "checkpoints",
    "weights",
    ".huggingface",
    "huggingface_hub",
    "wandb",
    "mlruns",
    "lightning_logs",
];

#[derive(Debug, Clone)]
pub struct NormalizedGlobalExcludes {
    dir_names: Vec<String>,
    dir_suffixes: Vec<String>,
    file_names: Vec<String>,
    file_suffixes: Vec<String>,
    path_segments: Vec<String>,
}

pub fn normalize_global_excludes(excludes: &GlobalExcludes) -> NormalizedGlobalExcludes {
    let normalized_dir_names = normalize_names(&excludes.dir_names);
    let normalized_dir_suffixes = normalize_suffixes(&excludes.dir_suffixes);
    let normalized_file_names = normalize_names(&excludes.file_names);
    let normalized_file_suffixes = normalize_suffixes(&excludes.extensions);
    let normalized_path_segments = normalize_patterns(&excludes.patterns);

    NormalizedGlobalExcludes {
        dir_names: normalized_dir_names,
        dir_suffixes: normalized_dir_suffixes,
        file_names: normalized_file_names,
        file_suffixes: normalized_file_suffixes,
        path_segments: normalized_path_segments,
    }
}

pub fn is_globally_excluded_dir_name(name: &str, excludes: &NormalizedGlobalExcludes) -> bool {
    let lowered = name.to_lowercase();
    if excludes.dir_names.iter().any(|value| value == &lowered) {
        return true;
    }
    matches_suffix(&lowered, &excludes.dir_suffixes)
}

pub fn is_globally_excluded_file_name(name: &str, excludes: &NormalizedGlobalExcludes) -> bool {
    let lowered = name.to_lowercase();
    if excludes.file_names.iter().any(|value| value == &lowered) {
        return true;
    }
    matches_suffix(&lowered, &excludes.file_suffixes)
}

pub fn is_globally_excluded_path(archive_path: &str, excludes: &NormalizedGlobalExcludes) -> bool {
    let lowered_path = archive_path.to_lowercase();
    for pattern in &excludes.path_segments {
        if matches_pattern(&lowered_path, pattern) {
            return true;
        }
    }
    false
}

fn normalize_suffixes(values: &[String]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return None;
            }
            let lowered = trimmed.to_lowercase();
            if lowered == "~" {
                return Some("~".to_string());
            }
            let normalized = if lowered.starts_with('.') {
                lowered
            } else {
                format!(".{lowered}")
            };
            Some(normalized)
        })
        .collect()
}

fn normalize_names(values: &[String]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return None;
            }
            Some(trimmed.to_lowercase())
        })
        .collect()
}

fn normalize_patterns(values: &[String]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return None;
            }
            let normalized = trimmed.trim_matches('/').to_lowercase();
            if normalized.is_empty() {
                return None;
            }
            Some(normalized)
        })
        .collect()
}

fn matches_pattern(path: &str, pattern: &str) -> bool {
    path == pattern
        || path.starts_with(&format!("{pattern}/"))
        || path.ends_with(&format!("/{pattern}"))
        || path.contains(&format!("/{pattern}/"))
}

fn matches_suffix(name: &str, suffixes: &[String]) -> bool {
    suffixes.iter().any(|suffix| {
        if suffix == "~" {
            name.ends_with('~')
        } else {
            name.ends_with(suffix)
        }
    })
}

pub fn recommended_global_excludes() -> GlobalExcludes {
    GlobalExcludes {
        dir_names: RECOMMENDED_DIR_NAMES
            .iter()
            .map(|value| value.to_string())
            .collect(),
        dir_suffixes: RECOMMENDED_DIR_SUFFIXES
            .iter()
            .map(|value| value.to_string())
            .collect(),
        file_names: RECOMMENDED_FILE_NAMES
            .iter()
            .map(|value| value.to_string())
            .collect(),
        extensions: RECOMMENDED_FILE_SUFFIXES
            .iter()
            .map(|value| value.to_string())
            .collect(),
        patterns: RECOMMENDED_PATH_SEGMENTS
            .iter()
            .map(|value| value.to_string())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BundleGitInfo, BundlePlan, BundleSelection, GlobalExcludes};
    use crate::progress::ProgressEmitter;
    use crate::scanner::scan_bundle;
    use tempfile::tempdir;

    #[test]
    fn normalizes_excludes() {
        let excludes = GlobalExcludes {
            dir_names: vec![],
            dir_suffixes: vec![],
            file_names: vec![],
            extensions: vec!["CKPT".to_string(), ".PT".to_string()],
            patterns: vec!["/Models/".to_string(), "  wandb  ".to_string()],
        };

        let normalized = normalize_global_excludes(&excludes);
        assert!(normalized.file_suffixes.contains(&".ckpt".to_string()));
        assert!(normalized.file_suffixes.contains(&".pt".to_string()));
        assert!(normalized.path_segments.contains(&"models".to_string()));
        assert!(normalized.path_segments.contains(&"wandb".to_string()));
    }

    #[test]
    fn global_excludes_filter_known_artifacts() {
        let dir = tempdir().unwrap();
        let repo_root = dir.path();

        std::fs::create_dir(repo_root.join("app")).unwrap();
        std::fs::create_dir(repo_root.join("models")).unwrap();

        std::fs::write(repo_root.join("README.md"), "root").unwrap();
        std::fs::write(repo_root.join("model.safetensors"), "weights").unwrap();
        std::fs::write(repo_root.join("app/index.ts"), "code").unwrap();
        std::fs::write(repo_root.join("app/weights.ckpt"), "checkpoint").unwrap();
        std::fs::write(repo_root.join("models/data.bin"), "data").unwrap();

        let plan = BundlePlan {
            output_path: repo_root.join("out.zip"),
            repo_root: repo_root.to_path_buf(),
            repo_id: "repo".to_string(),
            preset_id: "full".to_string(),
            preset_name: "Full".to_string(),
            selection: BundleSelection {
                include_root: true,
                top_level_dirs: vec!["app".to_string(), "models".to_string()],
                excluded_subdirs: vec![],
            },
            git: BundleGitInfo {
                head_sha: None,
                short_sha: None,
                branch: None,
            },
            built_at_iso: "2026-01-31T00:00:00Z".to_string(),
            global_excludes: GlobalExcludes {
                dir_names: vec![],
                dir_suffixes: vec![],
                file_names: vec![],
                extensions: vec![
                    ".safetensors".to_string(),
                    ".ckpt".to_string(),
                    ".bin".to_string(),
                ],
                patterns: vec!["models".to_string()],
            },
        };

        let mut progress = ProgressEmitter::new();
        let result = scan_bundle(&plan, &mut progress).unwrap();

        let archive_paths: std::collections::HashSet<_> = result
            .entries
            .iter()
            .map(|entry| entry.archive_path.as_str())
            .collect();

        assert!(archive_paths.contains("README.md"));
        assert!(archive_paths.contains("app/index.ts"));
        assert!(!archive_paths.contains("model.safetensors"));
        assert!(!archive_paths.contains("app/weights.ckpt"));
        assert!(!archive_paths.contains("models/data.bin"));
    }

    #[test]
    fn excluded_top_level_dirs_are_not_reported_as_included() {
        let dir = tempdir().unwrap();
        let repo_root = dir.path();

        std::fs::create_dir(repo_root.join("app")).unwrap();
        std::fs::create_dir(repo_root.join("models")).unwrap();
        std::fs::write(repo_root.join("app/index.ts"), "code").unwrap();
        std::fs::write(repo_root.join("models/data.bin"), "data").unwrap();

        let plan = BundlePlan {
            output_path: repo_root.join("out.zip"),
            repo_root: repo_root.to_path_buf(),
            repo_id: "repo".to_string(),
            preset_id: "full".to_string(),
            preset_name: "Full".to_string(),
            selection: BundleSelection {
                include_root: false,
                top_level_dirs: vec!["app".to_string(), "models".to_string()],
                excluded_subdirs: vec![],
            },
            git: BundleGitInfo {
                head_sha: None,
                short_sha: None,
                branch: None,
            },
            built_at_iso: "2026-01-31T00:00:00Z".to_string(),
            global_excludes: GlobalExcludes {
                dir_names: vec![],
                dir_suffixes: vec![],
                file_names: vec![],
                extensions: vec![],
                patterns: vec!["models".to_string()],
            },
        };

        let mut progress = ProgressEmitter::new();
        let result = scan_bundle(&plan, &mut progress).unwrap();
        assert_eq!(result.top_level_dirs_included, vec!["app".to_string()]);
    }
}
