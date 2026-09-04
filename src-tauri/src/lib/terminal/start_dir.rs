// Path: src-tauri/src/lib/terminal/start_dir.rs
// Description: Maps a repo root to the directory pwsh starts in and the WSL entry command it runs for a native WSL root

use crate::config::types::RepoRoot;
use crate::paths::wsl_convert::wsl_to_windows_path;
use crate::wsl_control::resolve_native_root;
use std::path::{Path, PathBuf};

/// Where the shell starts and what it runs first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartDir {
    pub cwd: PathBuf,
    /// `wsl.exe [-d <distro>] --cd '<path>'` for a native WSL root; `None` when
    /// the shell simply starts in `cwd`
    pub initial_command: Option<String>,
}

/// The two shapes a root resolves to, before the directory is checked.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Target {
    /// Start pwsh in this host directory
    Host(String),
    /// Start pwsh in the user profile and enter the distro at this Linux path
    WslEntry(String),
}

/// A host root, or a WSL root on a mounted Windows drive, starts the shell in
/// that directory. A native WSL root starts pwsh in the user profile (so the
/// profile loads as it always does) and enters the distro at the repo path.
pub fn resolve(root: &RepoRoot, distro: Option<&str>) -> Result<StartDir, String> {
    match target(root) {
        Target::Host(path) => Ok(StartDir {
            cwd: existing_dir(Path::new(&path))?,
            initial_command: None,
        }),
        Target::WslEntry(path) => {
            let resolved = resolve_native_root(distro, &path)?;
            let home = dirs::home_dir().ok_or_else(|| {
                "Failed to resolve the user profile directory for the WSL terminal".to_string()
            })?;
            Ok(StartDir {
                cwd: existing_dir(&home)?,
                initial_command: Some(wsl_entry_command(&resolved.path, &resolved.distro)),
            })
        }
    }
}

fn target(root: &RepoRoot) -> Target {
    match root {
        RepoRoot::Host { path } => Target::Host(path.clone()),
        RepoRoot::Wsl { path } => match wsl_to_windows_path(path) {
            Some(host_path) => Target::Host(host_path),
            None => Target::WslEntry(path.clone()),
        },
    }
}

/// portable-pty silently falls back to the user profile when the cwd is
/// missing, so a missing directory is refused here instead (ADR-008).
fn existing_dir(path: &Path) -> Result<PathBuf, String> {
    if path.is_dir() {
        Ok(path.to_path_buf())
    } else {
        Err(format!(
            "Terminal start directory does not exist: {}",
            path.display()
        ))
    }
}

/// The preflight's exact distro is always pinned. A later entry failure exits
/// pwsh instead of presenting a misleading prompt in the profile directory;
/// a successful interactive bash exit still returns to pwsh.
pub fn wsl_entry_command(path: &str, distro: &str) -> String {
    format!(
        "wsl.exe -d {} --cd {}; if ($LASTEXITCODE -ne 0) {{ exit $LASTEXITCODE }}",
        pwsh_single_quote(distro),
        pwsh_single_quote(path)
    )
}

/// Inside pwsh single quotes the only escape is doubling the quote.
fn pwsh_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::{resolve, target, wsl_entry_command, Target};
    use crate::config::types::RepoRoot;
    use tempfile::tempdir;

    #[test]
    fn a_host_root_starts_in_place() {
        let root = RepoRoot::Host {
            path: r"C:\dev\repo".to_string(),
        };
        assert_eq!(target(&root), Target::Host(r"C:\dev\repo".to_string()));
    }

    #[test]
    fn a_wsl_root_on_a_windows_drive_starts_in_the_windows_directory() {
        let root = RepoRoot::Wsl {
            path: "/mnt/c/dev/repo".to_string(),
        };
        assert_eq!(target(&root), Target::Host(r"C:\dev\repo".to_string()));
    }

    #[test]
    fn a_native_wsl_root_enters_the_distro() {
        let root = RepoRoot::Wsl {
            path: "/home/johnf/code/repo".to_string(),
        };
        assert_eq!(
            target(&root),
            Target::WslEntry("/home/johnf/code/repo".to_string())
        );
    }

    #[test]
    fn the_entry_command_pins_the_distro_and_fails_pwsh_on_entry_error() {
        assert_eq!(
            wsl_entry_command("/home/johnf/it's here", "Ubuntu-22.04"),
            "wsl.exe -d 'Ubuntu-22.04' --cd '/home/johnf/it''s here'; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }"
        );
    }

    #[test]
    fn a_missing_start_directory_is_refused() {
        let dir = tempdir().expect("tempdir");
        let present = RepoRoot::Host {
            path: dir.path().to_string_lossy().into_owned(),
        };
        let resolved = resolve(&present, None).expect("resolve");
        assert_eq!(resolved.cwd, dir.path());
        assert_eq!(resolved.initial_command, None);

        let missing = RepoRoot::Host {
            path: dir.path().join("absent").to_string_lossy().into_owned(),
        };
        let err = resolve(&missing, None).expect_err("missing dir");
        assert!(err.contains("does not exist"), "{err}");
    }
}
