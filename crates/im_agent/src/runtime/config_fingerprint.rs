// Path: crates/im_agent/src/runtime/config_fingerprint.rs
// Description: Compute watcher-relevant config fingerprint

use serde::Serialize;

use super::config::{AppConfig, RepoConfig};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RepoFingerprint {
    repo_id: String,
    root_kind: String,
    root_path: String,
    docs_globs: Vec<String>,
    code_globs: Vec<String>,
    ignore_globs: Vec<String>,
    auto_stage: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FingerprintData {
    staging_root: String,
    recent_files_limit: usize,
    classification_excludes: ClassificationExcludesFingerprint,
    repos: Vec<RepoFingerprint>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClassificationExcludesFingerprint {
    dir_names: Vec<String>,
    dir_suffixes: Vec<String>,
    file_names: Vec<String>,
    extensions: Vec<String>,
    patterns: Vec<String>,
}

pub fn compute_config_fingerprint(config: &AppConfig, staging_root: &str) -> String {
    let mut repos: Vec<RepoFingerprint> = config.repos.iter().map(|repo| map_repo(repo)).collect();

    repos.sort_by(|a, b| a.repo_id.cmp(&b.repo_id));

    let data = FingerprintData {
        staging_root: staging_root.to_string(),
        recent_files_limit: config.recent_files_limit,
        classification_excludes: map_classification_excludes(config),
        repos,
    };

    serde_json::to_string(&data).unwrap_or_default()
}

fn map_classification_excludes(config: &AppConfig) -> ClassificationExcludesFingerprint {
    let mut dir_names = config.classification_excludes.dir_names.clone();
    dir_names.sort();
    let mut dir_suffixes = config.classification_excludes.dir_suffixes.clone();
    dir_suffixes.sort();
    let mut file_names = config.classification_excludes.file_names.clone();
    file_names.sort();
    let mut extensions = config.classification_excludes.extensions.clone();
    extensions.sort();
    let mut patterns = config.classification_excludes.patterns.clone();
    patterns.sort();

    ClassificationExcludesFingerprint {
        dir_names,
        dir_suffixes,
        file_names,
        extensions,
        patterns,
    }
}

fn map_repo(repo: &RepoConfig) -> RepoFingerprint {
    let mut docs_globs = repo.docs_globs.clone();
    docs_globs.sort();
    let mut code_globs = repo.code_globs.clone();
    code_globs.sort();
    let mut ignore_globs = repo.ignore_globs.clone();
    ignore_globs.sort();

    RepoFingerprint {
        repo_id: repo.repo_id.clone(),
        root_kind: repo.root.kind().to_string(),
        root_path: repo.root.path().to_string(),
        docs_globs,
        code_globs,
        ignore_globs,
        auto_stage: repo.auto_stage,
    }
}
