// Path: crates/im_agent/src/runtime/config_fingerprint.rs
// Description: Compute watcher-relevant config fingerprint

use serde::Serialize;

use super::config::{AppConfig, RepoConfig};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RepoFingerprint {
    repo_id: String,
    wsl_path: String,
    docs_globs: Vec<String>,
    code_globs: Vec<String>,
    ignore_globs: Vec<String>,
    auto_stage: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FingerprintData {
    staging_wsl_root: String,
    recent_files_limit: usize,
    repos: Vec<RepoFingerprint>,
}

pub fn compute_config_fingerprint(config: &AppConfig, staging_wsl_root: &str) -> String {
    let mut repos: Vec<RepoFingerprint> = config
        .repos
        .iter()
        .map(|repo| map_repo(repo))
        .collect();

    repos.sort_by(|a, b| a.repo_id.cmp(&b.repo_id));

    let data = FingerprintData {
        staging_wsl_root: staging_wsl_root.to_string(),
        recent_files_limit: config.recent_files_limit,
        repos,
    };

    serde_json::to_string(&data).unwrap_or_default()
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
        wsl_path: repo.wsl_path.clone(),
        docs_globs,
        code_globs,
        ignore_globs,
        auto_stage: repo.auto_stage,
    }
}
