// Path: crates/im_agent/src/runtime/config_fingerprint.rs
// Description: Compute global and per-repo watcher-relevant config fingerprints

use std::collections::HashMap;

use serde::Serialize;

use super::config::{AppConfig, RepoConfig};

/// Watcher-relevant config split so a hello restarts only what changed: `global`
/// feeds every watcher, `repos` holds one fingerprint per repo id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigFingerprint {
    pub global: String,
    pub repos: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RepoFingerprint<'a> {
    root_kind: &'a str,
    root_path: &'a str,
    docs_globs: Vec<String>,
    code_globs: Vec<String>,
    ignore_globs: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GlobalFingerprint {
    staging_root: String,
    recent_files_limit: usize,
    classification_excludes: ClassificationExcludesFingerprint,
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

pub fn compute_config_fingerprint(config: &AppConfig, staging_root: &str) -> ConfigFingerprint {
    let global = GlobalFingerprint {
        staging_root: staging_root.to_string(),
        recent_files_limit: config.recent_files_limit,
        classification_excludes: map_classification_excludes(config),
    };
    let repos = config
        .repos
        .iter()
        .map(|repo| (repo.repo_id.clone(), fingerprint_repo(repo)))
        .collect();
    ConfigFingerprint {
        global: serde_json::to_string(&global).unwrap_or_default(),
        repos,
    }
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

fn fingerprint_repo(repo: &RepoConfig) -> String {
    let mut docs_globs = repo.docs_globs.clone();
    docs_globs.sort();
    let mut code_globs = repo.code_globs.clone();
    code_globs.sort();
    let mut ignore_globs = repo.ignore_globs.clone();
    ignore_globs.sort();

    let data = RepoFingerprint {
        root_kind: repo.root.kind(),
        root_path: repo.root.path(),
        docs_globs,
        code_globs,
        ignore_globs,
    };
    serde_json::to_string(&data).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn fingerprint(config: serde_json::Value) -> ConfigFingerprint {
        let config: AppConfig = serde_json::from_value(config).expect("config");
        compute_config_fingerprint(&config, "/staging")
    }

    fn repo(repo_id: &str, ignore: &[&str]) -> serde_json::Value {
        json!({"repoId": repo_id, "root": {"kind": "wsl", "path": format!("/r/{repo_id}")},
            "ignoreGlobs": ignore})
    }

    #[test]
    fn repo_change_moves_only_that_repo_fingerprint() {
        let before = fingerprint(json!({"repos": [repo("a", &[]), repo("b", &[])]}));
        let after = fingerprint(json!({"repos": [repo("a", &["**/*.tmp"]), repo("b", &[])]}));
        assert_eq!(before.global, after.global);
        assert_ne!(before.repos["a"], after.repos["a"]);
        assert_eq!(before.repos["b"], after.repos["b"]);

        let excluded = fingerprint(json!({"repos": [repo("a", &[]), repo("b", &[])],
            "classificationExcludes": {"dirNames": ["vendor"]}}));
        assert_ne!(before.global, excluded.global);
    }
}
