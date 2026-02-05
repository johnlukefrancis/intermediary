// Path: crates/im_agent/src/runtime/config.rs
// Description: Minimal app configuration structures for the agent runtime

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoRootKind {
    Wsl,
    Windows,
}

impl RepoRootKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Wsl => "wsl",
            Self::Windows => "windows",
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
    pub repos: Vec<RepoConfig>,
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
            RepoRoot::Windows { .. } => RepoRootKind::Windows,
        }
    }

    pub fn wsl_root_path(&self) -> Option<&str> {
        self.root.wsl_path()
    }

    pub fn windows_root_path(&self) -> Option<&str> {
        self.root.windows_path()
    }

    pub fn root_path_for_kind(&self, kind: RepoRootKind) -> Option<&str> {
        self.root.path_for_kind(kind)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RepoRoot {
    Wsl { path: String },
    Windows { path: String },
}

impl RepoRoot {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Wsl { .. } => "wsl",
            Self::Windows { .. } => "windows",
        }
    }

    pub fn path(&self) -> &str {
        match self {
            Self::Wsl { path } | Self::Windows { path } => path,
        }
    }

    pub fn wsl_path(&self) -> Option<&str> {
        match self {
            Self::Wsl { path } => Some(path),
            Self::Windows { .. } => None,
        }
    }

    pub fn windows_path(&self) -> Option<&str> {
        match self {
            Self::Wsl { .. } => None,
            Self::Windows { path } => Some(path),
        }
    }

    pub fn path_for_kind(&self, kind: RepoRootKind) -> Option<&str> {
        match kind {
            RepoRootKind::Wsl => self.wsl_path(),
            RepoRootKind::Windows => self.windows_path(),
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

fn default_bundle_presets() -> Vec<BundlePreset> {
    vec![BundlePreset {
        preset_id: "context".to_string(),
        preset_name: "Context".to_string(),
        include_root: true,
        top_level_dirs: Vec::new(),
    }]
}
