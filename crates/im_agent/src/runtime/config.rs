// Path: crates/im_agent/src/runtime/config.rs
// Description: Minimal app configuration structures for the agent runtime

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoRootKind {
    Wsl,
    Host,
}

impl RepoRootKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Wsl => "wsl",
            Self::Host => "host",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub agent_host: Option<String>,
    pub agent_port: Option<u16>,
    #[serde(default = "default_true")]
    pub auto_stage_global: bool,
    #[serde(default = "default_recent_files_limit")]
    pub recent_files_limit: usize,
    #[serde(default)]
    pub classification_excludes: ClassificationExcludes,
    #[serde(default)]
    pub repos: Vec<RepoConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationExcludes {
    #[serde(default)]
    pub dir_names: Vec<String>,
    #[serde(default)]
    pub dir_suffixes: Vec<String>,
    #[serde(default)]
    pub file_names: Vec<String>,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub patterns: Vec<String>,
}

impl ClassificationExcludes {
    pub fn to_ignore_globs(&self) -> Vec<String> {
        let mut globs = Vec::new();

        for value in &self.dir_names {
            let normalized = normalize_segment(value);
            if !normalized.is_empty() {
                globs.push(format!("**/{normalized}/**"));
            }
        }

        for value in &self.dir_suffixes {
            let normalized = normalize_extension(value);
            if !normalized.is_empty() {
                globs.push(format!("**/*{normalized}/**"));
            }
        }

        for value in &self.file_names {
            let normalized = normalize_name(value);
            if !normalized.is_empty() {
                globs.push(format!("**/{normalized}"));
            }
        }

        for value in &self.extensions {
            let normalized = normalize_extension(value);
            if !normalized.is_empty() {
                globs.push(format!("**/*{normalized}"));
            }
        }

        for value in &self.patterns {
            let normalized = normalize_segment(value);
            if !normalized.is_empty() {
                globs.push(format!("**/{normalized}/**"));
            }
        }

        globs
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoConfig {
    pub repo_id: String,
    pub root: RepoRoot,
    #[serde(default = "default_true")]
    pub auto_stage: bool,
    #[serde(default)]
    pub docs_globs: Vec<String>,
    #[serde(default)]
    pub code_globs: Vec<String>,
    #[serde(default)]
    pub ignore_globs: Vec<String>,
    #[serde(default = "default_bundle_presets")]
    pub bundle_presets: Vec<BundlePreset>,
}

impl RepoConfig {
    pub fn root_kind(&self) -> RepoRootKind {
        match self.root {
            RepoRoot::Wsl { .. } => RepoRootKind::Wsl,
            RepoRoot::Host { .. } => RepoRootKind::Host,
        }
    }

    pub fn wsl_root_path(&self) -> Option<&str> {
        self.root.wsl_path()
    }

    pub fn host_root_path(&self) -> Option<&str> {
        self.root.host_path()
    }

    pub fn root_path_for_kind(&self, kind: RepoRootKind) -> Option<&str> {
        self.root.path_for_kind(kind)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RepoRoot {
    Wsl { path: String },
    #[serde(alias = "windows")]
    Host { path: String },
}

impl RepoRoot {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Wsl { .. } => "wsl",
            Self::Host { .. } => "host",
        }
    }

    pub fn path(&self) -> &str {
        match self {
            Self::Wsl { path } | Self::Host { path } => path,
        }
    }

    pub fn wsl_path(&self) -> Option<&str> {
        match self {
            Self::Wsl { path } => Some(path),
            Self::Host { .. } => None,
        }
    }

    pub fn host_path(&self) -> Option<&str> {
        match self {
            Self::Wsl { .. } => None,
            Self::Host { path } => Some(path),
        }
    }

    pub fn path_for_kind(&self, kind: RepoRootKind) -> Option<&str> {
        match kind {
            RepoRootKind::Wsl => self.wsl_path(),
            RepoRootKind::Host => self.host_path(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundlePreset {
    pub preset_id: String,
    pub preset_name: String,
    #[serde(default = "default_true")]
    pub include_root: bool,
    #[serde(default)]
    pub top_level_dirs: Vec<String>,
}

fn default_true() -> bool {
    true
}

fn default_recent_files_limit() -> usize {
    200
}

fn normalize_segment(value: &str) -> String {
    value.trim().trim_matches('/').to_string()
}

fn normalize_extension(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed == "~" {
        return "~".to_string();
    }
    if trimmed.starts_with('.') {
        return trimmed.to_string();
    }
    format!(".{trimmed}")
}

fn normalize_name(value: &str) -> String {
    value.trim().to_string()
}

fn default_bundle_presets() -> Vec<BundlePreset> {
    vec![BundlePreset {
        preset_id: "context".to_string(),
        preset_name: "Context".to_string(),
        include_root: true,
        top_level_dirs: Vec::new(),
    }]
}
