// Path: crates/im_bundle/src/global_excludes.rs
// Description: Normalize and apply user-configurable global excludes for bundle scanning

use crate::plan::GlobalExcludes;

const MODEL_WEIGHT_EXTENSIONS: &[&str] = &[
    ".safetensors",
    ".ckpt",
    ".pt",
    ".pth",
    ".bin",
];

const MODEL_FORMAT_EXTENSIONS: &[&str] = &[
    ".onnx",
    ".pb",
    ".h5",
    ".keras",
];

const MODEL_DIR_PATTERNS: &[&str] = &[
    "models",
    "checkpoints",
    "weights",
];

const HF_CACHE_PATTERNS: &[&str] = &[".huggingface", "huggingface_hub"];

const EXPERIMENT_PATTERNS: &[&str] = &["wandb", "mlruns", "lightning_logs"];

#[derive(Debug, Clone)]
pub struct NormalizedGlobalExcludes {
    extensions: Vec<String>,
    patterns: Vec<String>,
}

pub fn normalize_global_excludes(excludes: &GlobalExcludes) -> NormalizedGlobalExcludes {
    let mut extensions = Vec::new();
    let mut patterns = Vec::new();

    if excludes.presets.model_weights {
        extensions.extend(MODEL_WEIGHT_EXTENSIONS.iter().map(|ext| ext.to_string()));
    }
    if excludes.presets.model_formats {
        extensions.extend(MODEL_FORMAT_EXTENSIONS.iter().map(|ext| ext.to_string()));
    }
    if excludes.presets.model_dirs {
        patterns.extend(MODEL_DIR_PATTERNS.iter().map(|pattern| pattern.to_string()));
    }
    if excludes.presets.hf_caches {
        patterns.extend(HF_CACHE_PATTERNS.iter().map(|pattern| pattern.to_string()));
    }
    if excludes.presets.experiment_logs {
        patterns.extend(EXPERIMENT_PATTERNS.iter().map(|pattern| pattern.to_string()));
    }

    extensions.extend(excludes.extensions.iter().cloned());
    patterns.extend(excludes.patterns.iter().cloned());

    let normalized_extensions = normalize_extensions(&extensions);
    let normalized_patterns = normalize_patterns(&patterns);

    NormalizedGlobalExcludes {
        extensions: normalized_extensions,
        patterns: normalized_patterns,
    }
}

pub fn is_globally_excluded_file(archive_path: &str, excludes: &NormalizedGlobalExcludes) -> bool {
    let lowered_path = archive_path.to_lowercase();

    for ext in &excludes.extensions {
        if lowered_path.ends_with(ext) {
            return true;
        }
    }

    for pattern in &excludes.patterns {
        if matches_pattern(&lowered_path, pattern) {
            return true;
        }
    }

    false
}

pub fn is_globally_excluded_dir(archive_path: &str, excludes: &NormalizedGlobalExcludes) -> bool {
    let lowered_path = archive_path.to_lowercase();
    for pattern in &excludes.patterns {
        if matches_pattern(&lowered_path, pattern) {
            return true;
        }
    }
    false
}

fn normalize_extensions(values: &[String]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return None;
            }
            let lowered = trimmed.to_lowercase();
            let normalized = if lowered.starts_with('.') {
                lowered
            } else {
                format!(".{lowered}")
            };
            Some(normalized)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BundleGitInfo, BundlePlan, BundleSelection, GlobalExcludes, GlobalExcludePresets};
    use crate::scanner::scan_bundle;
    use crate::progress::ProgressEmitter;
    use tempfile::tempdir;

    #[test]
    fn normalizes_extensions_and_patterns() {
        let excludes = GlobalExcludes {
            presets: GlobalExcludePresets {
                model_weights: false,
                model_formats: false,
                model_dirs: false,
                hf_caches: false,
                experiment_logs: false,
            },
            extensions: vec!["CKPT".to_string(), ".PT".to_string()],
            patterns: vec!["/Models/".to_string(), "  wandb  ".to_string()],
        };

        let normalized = normalize_global_excludes(&excludes);
        assert!(normalized.extensions.contains(&".ckpt".to_string()));
        assert!(normalized.extensions.contains(&".pt".to_string()));
        assert!(normalized.patterns.contains(&"models".to_string()));
        assert!(normalized.patterns.contains(&"wandb".to_string()));
    }

    #[test]
    fn ml_artifact_preset_filters_known_artifacts() {
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
            global_excludes: GlobalExcludes::default(),
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
            global_excludes: GlobalExcludes::default(),
        };

        let mut progress = ProgressEmitter::new();
        let result = scan_bundle(&plan, &mut progress).unwrap();
        assert_eq!(result.top_level_dirs_included, vec!["app".to_string()]);
    }
}
