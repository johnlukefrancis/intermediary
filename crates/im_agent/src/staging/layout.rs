// Path: crates/im_agent/src/staging/layout.rs
// Description: Central staging layout derivation for file and bundle outputs

use std::path::{Path, PathBuf};

use crate::error::AgentError;

#[derive(Debug, Clone)]
pub struct PathBridgeConfig {
    pub staging_host_root: String,
    pub staging_wsl_root: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StagingRootKind {
    Wsl,
    Host,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagingPathViews {
    pub host_path: String,
    pub wsl_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedPaths {
    pub runtime_path: PathBuf,
    pub host_path: String,
    pub wsl_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StagingLayout {
    runtime_root: PathBuf,
    root_kind: StagingRootKind,
    host_root_is_windows: bool,
}

impl StagingLayout {
    pub fn from_config(
        config: &PathBridgeConfig,
        root_kind: StagingRootKind,
    ) -> Result<Self, AgentError> {
        let runtime_root = match root_kind {
            StagingRootKind::Wsl => config.staging_wsl_root.as_deref().ok_or_else(|| {
                AgentError::new(
                    "MISSING_WSL_ROOT",
                    "WSL staging root is required for WSL staging kind",
                )
            })?,
            StagingRootKind::Host => config.staging_host_root.as_str(),
        };

        Ok(Self {
            runtime_root: PathBuf::from(runtime_root),
            root_kind,
            host_root_is_windows: is_windows_style_path(&config.staging_host_root),
        })
    }

    pub fn runtime_root(&self) -> &Path {
        &self.runtime_root
    }

    pub fn files_root(&self) -> PathBuf {
        self.runtime_root.join("files")
    }

    pub fn bundles_root(&self) -> PathBuf {
        self.runtime_root.join("bundles")
    }

    pub fn bundles_dir(&self, repo_id: &str, preset_id: &str) -> PathBuf {
        self.bundles_root().join(repo_id).join(preset_id)
    }

    pub fn file_paths(
        &self,
        repo_id: &str,
        relative_path: &str,
    ) -> Result<StagedPaths, AgentError> {
        let runtime_path = self.files_root().join(repo_id).join(relative_path);
        self.paths_for_runtime_path(runtime_path)
    }

    pub fn path_views_for_runtime_path(
        &self,
        runtime_path: &Path,
    ) -> Result<StagingPathViews, AgentError> {
        let runtime_raw = runtime_path.to_string_lossy().to_string();
        match self.root_kind {
            StagingRootKind::Wsl => Ok(StagingPathViews {
                host_path: wsl_to_windows(&runtime_raw)?,
                wsl_path: Some(runtime_raw),
            }),
            StagingRootKind::Host => {
                let host_path = if self.host_root_is_windows {
                    runtime_raw.replace('/', "\\")
                } else {
                    runtime_raw
                };
                Ok(StagingPathViews {
                    host_path,
                    wsl_path: None,
                })
            }
        }
    }

    fn paths_for_runtime_path(&self, runtime_path: PathBuf) -> Result<StagedPaths, AgentError> {
        let views = self.path_views_for_runtime_path(&runtime_path)?;
        Ok(StagedPaths {
            runtime_path,
            host_path: views.host_path,
            wsl_path: views.wsl_path,
        })
    }
}

pub fn wsl_to_windows(wsl_path: &str) -> Result<String, AgentError> {
    let stripped = wsl_path.strip_prefix("/mnt/").ok_or_else(|| {
        AgentError::new(
            "INVALID_PATH",
            format!("Invalid WSL path for Windows conversion: {wsl_path}"),
        )
    })?;

    let mut parts = stripped.splitn(2, '/');
    let drive = parts.next().unwrap_or_default();
    let rest = parts.next().unwrap_or("");

    if drive.len() != 1 {
        return Err(AgentError::new(
            "INVALID_PATH",
            format!("Invalid WSL path for Windows conversion: {wsl_path}"),
        ));
    }

    let drive = drive
        .chars()
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let windows_rest = rest.replace('/', "\\");
    Ok(format!("{drive}:\\{windows_rest}"))
}

pub fn windows_to_wsl(windows_path: &str) -> Option<String> {
    let normalized = windows_path.trim().replace('/', "\\");
    let bytes = normalized.as_bytes();
    if bytes.len() < 2 || bytes[1] != b':' {
        return None;
    }

    let drive = (bytes[0] as char).to_ascii_lowercase();
    if !drive.is_ascii_alphabetic() {
        return None;
    }

    let suffix = normalized[2..].trim_start_matches('\\').replace('\\', "/");
    if suffix.is_empty() {
        return Some(format!("/mnt/{drive}"));
    }
    Some(format!("/mnt/{drive}/{suffix}"))
}

fn is_windows_style_path(path: &str) -> bool {
    if path.starts_with("\\\\") {
        return true;
    }
    let bytes = path.as_bytes();
    (bytes.len() >= 2 && bytes[1] == b':' && (bytes[0] as char).is_ascii_alphabetic())
        || path.contains('\\')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wsl_root_is_required_for_wsl_layout() {
        let config = PathBridgeConfig {
            staging_host_root: "C:\\staging".to_string(),
            staging_wsl_root: None,
        };
        let err = StagingLayout::from_config(&config, StagingRootKind::Wsl)
            .expect_err("expected missing WSL root error");
        assert_eq!(err.code(), "MISSING_WSL_ROOT");
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn posix_host_root_produces_posix_paths() {
        let root = "/Users/dev/Library/Application Support/Intermediary/staging";
        let config = PathBridgeConfig {
            staging_host_root: root.to_string(),
            staging_wsl_root: None,
        };

        let layout = StagingLayout::from_config(&config, StagingRootKind::Host)
            .expect("host layout should resolve");
        let staged = layout
            .file_paths("textureportal", "docs/prd.md")
            .expect("file paths should resolve");

        let expected = PathBuf::from(root)
            .join("files")
            .join("textureportal")
            .join("docs/prd.md");

        assert_eq!(staged.runtime_path, expected);
        assert_eq!(staged.host_path, expected.to_string_lossy().to_string());
        assert!(staged.wsl_path.is_none());
        assert!(!staged.host_path.contains('\\'));

        let bundle_dir = layout.bundles_dir("textureportal", "context");
        assert_eq!(
            bundle_dir,
            PathBuf::from(root)
                .join("bundles")
                .join("textureportal")
                .join("context")
        );
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn windows_host_root_still_produces_windows_host_paths() {
        let root = "C:\\Users\\dev\\AppData\\Local\\Intermediary\\staging";
        let config = PathBridgeConfig {
            staging_host_root: root.to_string(),
            staging_wsl_root: None,
        };

        let layout = StagingLayout::from_config(&config, StagingRootKind::Host)
            .expect("host layout should resolve");
        let staged = layout
            .file_paths("textureportal", "src/main.ts")
            .expect("file paths should resolve");

        assert_eq!(
            staged.host_path,
            "C:\\Users\\dev\\AppData\\Local\\Intermediary\\staging\\files\\textureportal\\src\\main.ts"
        );
        assert!(staged.wsl_path.is_none());
        assert!(staged
            .runtime_path
            .to_string_lossy()
            .contains("/files/textureportal/src/main.ts"));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn wsl_layout_converts_runtime_path_to_host_path() {
        let config = PathBridgeConfig {
            staging_host_root: "C:\\Users\\dev\\AppData\\Local\\Intermediary\\staging".to_string(),
            staging_wsl_root: Some(
                "/mnt/c/Users/dev/AppData/Local/Intermediary/staging".to_string(),
            ),
        };

        let layout = StagingLayout::from_config(&config, StagingRootKind::Wsl)
            .expect("wsl layout should resolve");
        let staged = layout
            .file_paths("textureportal", "src/main.ts")
            .expect("file paths should resolve");

        assert_eq!(
            staged.runtime_path,
            PathBuf::from("/mnt/c/Users/dev/AppData/Local/Intermediary/staging")
                .join("files")
                .join("textureportal")
                .join("src/main.ts")
        );
        assert_eq!(
            staged.host_path,
            "C:\\Users\\dev\\AppData\\Local\\Intermediary\\staging\\files\\textureportal\\src\\main.ts"
        );
        assert_eq!(
            staged.wsl_path.as_deref(),
            Some("/mnt/c/Users/dev/AppData/Local/Intermediary/staging/files/textureportal/src/main.ts")
        );
    }
}
