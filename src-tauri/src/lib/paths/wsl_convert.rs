// Path: src-tauri/src/lib/paths/wsl_convert.rs
// Description: Windows <-> WSL path conversion utilities

#[cfg(target_os = "windows")]
use std::process::Command;

/// Convert a Windows path to its WSL equivalent
///
/// Example: `C:\Users\john\AppData` -> `/mnt/c/Users/john/AppData`
pub fn windows_to_wsl_path(windows_path: &str) -> Option<String> {
    // Already a WSL/Linux path.
    if windows_path.starts_with('/') {
        return Some(windows_path.replace('\\', "/"));
    }

    // Handle UNC paths like \\wsl$\Ubuntu\...
    if windows_path.starts_with(r"\\wsl$\") || windows_path.starts_with(r"\\wsl.localhost\") {
        // Extract path after distro name
        let rest = windows_path
            .strip_prefix(r"\\wsl$\")
            .or_else(|| windows_path.strip_prefix(r"\\wsl.localhost\"))?;

        // Find end of distro name (next backslash)
        let slash_pos = rest.find('\\')?;
        let unix_path = &rest[slash_pos..];
        return Some(unix_path.replace('\\', "/"));
    }

    // Handle standard Windows paths like C:\... or C:/...
    let mut chars = windows_path.chars();
    let drive_letter = chars.next()?.to_ascii_lowercase();

    // Verify it's a drive letter followed by :
    if !drive_letter.is_ascii_alphabetic() {
        return None;
    }
    if chars.next() != Some(':') {
        return None;
    }
    match chars.next() {
        Some('\\') | Some('/') => {
            let rest: String = chars.collect();
            if rest.is_empty() {
                return Some(format!("/mnt/{drive_letter}"));
            }
            let unix_rest = rest.replace('\\', "/");
            Some(format!("/mnt/{drive_letter}/{unix_rest}"))
        }
        None => Some(format!("/mnt/{drive_letter}")),
        Some(_) => None,
    }
}

/// Convert a WSL path to its Windows equivalent
///
/// Example: `/mnt/c/Users/john/AppData` -> `C:\Users\john\AppData`
pub fn wsl_to_windows_path(wsl_path: &str) -> Option<String> {
    // Must start with /mnt/X where X is a drive letter
    let stripped = wsl_path.strip_prefix("/mnt/")?;
    let mut chars = stripped.chars();
    let drive_letter = chars.next()?.to_ascii_uppercase();

    if !drive_letter.is_ascii_alphabetic() {
        return None;
    }

    // Next char should be / or end of string
    match chars.next() {
        Some('/') => {}
        None => return Some(format!("{drive_letter}:\\")),
        Some(_) => return None,
    }

    let rest: String = chars.collect();
    let windows_rest = rest.replace('/', "\\");

    Some(format!("{drive_letter}:\\{windows_rest}"))
}

/// Errors that can occur when running wslpath via subprocess.
#[derive(Debug)]
pub enum WslConvertError {
    /// WSL conversion is not available on this host OS
    UnsupportedPlatform,
    /// wsl.exe not found or failed to execute
    WslNotFound(std::io::Error),
    /// wslpath returned a nonzero exit code
    WslpathFailed {
        exit_code: Option<i32>,
        stderr: String,
    },
    /// wslpath returned empty or invalid output
    InvalidOutput(String),
}

impl std::fmt::Display for WslConvertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                write!(f, "WSL path conversion is only available on Windows hosts")
            }
            Self::WslNotFound(e) => write!(f, "WSL not found or failed to execute: {e}"),
            Self::WslpathFailed { exit_code, stderr } => {
                let code_str = exit_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                write!(f, "wslpath failed (exit code {code_str}): {stderr}")
            }
            Self::InvalidOutput(msg) => write!(f, "wslpath returned invalid output: {msg}"),
        }
    }
}

impl std::error::Error for WslConvertError {}

/// Convert a native WSL path to Windows format by calling `wsl.exe wslpath -w`.
///
/// This is a blocking function; call from `spawn_blocking` in async contexts.
///
/// Example: `/home/john/code` -> `\\wsl.localhost\<distro>\home\john\code`
#[cfg(target_os = "windows")]
pub fn run_wslpath(
    wsl_path: &str,
    distro_override: Option<&str>,
) -> Result<String, WslConvertError> {
    let mut command = Command::new("wsl.exe");
    for arg in build_wslpath_command_args(wsl_path, distro_override) {
        command.arg(arg);
    }
    let output = command.output().map_err(WslConvertError::WslNotFound)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(WslConvertError::WslpathFailed {
            exit_code: output.status.code(),
            stderr,
        });
    }

    let windows_path = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if windows_path.is_empty() {
        return Err(WslConvertError::InvalidOutput(
            "wslpath returned empty output".to_string(),
        ));
    }

    Ok(windows_path)
}

#[cfg(not(target_os = "windows"))]
pub fn run_wslpath(
    _wsl_path: &str,
    _distro_override: Option<&str>,
) -> Result<String, WslConvertError> {
    Err(WslConvertError::UnsupportedPlatform)
}

#[cfg(target_os = "windows")]
fn resolve_effective_wsl_distro(distro_override: Option<&str>) -> Option<String> {
    let explicit = distro_override
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(value) = explicit {
        return Some(value.to_string());
    }

    std::env::var("INTERMEDIARY_WSL_DISTRO")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(target_os = "windows")]
fn build_wslpath_command_args(wsl_path: &str, distro_override: Option<&str>) -> Vec<String> {
    let mut args = Vec::with_capacity(5);
    if let Some(distro) = resolve_effective_wsl_distro(distro_override) {
        args.push("-d".to_string());
        args.push(distro);
    }
    args.push("wslpath".to_string());
    args.push("-w".to_string());
    args.push(wsl_path.to_string());
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "windows")]
    use std::sync::{Mutex, OnceLock};

    #[test]
    fn test_windows_to_wsl() {
        assert_eq!(
            windows_to_wsl_path(r"C:\Users\john\AppData"),
            Some("/mnt/c/Users/john/AppData".to_string())
        );
        assert_eq!(
            windows_to_wsl_path(r"D:\code\project"),
            Some("/mnt/d/code/project".to_string())
        );
        assert_eq!(
            windows_to_wsl_path("C:/Users/john/AppData"),
            Some("/mnt/c/Users/john/AppData".to_string())
        );
        assert_eq!(windows_to_wsl_path("C:"), Some("/mnt/c".to_string()));
        assert_eq!(windows_to_wsl_path(r"C:\"), Some("/mnt/c".to_string()));
        assert_eq!(windows_to_wsl_path("C:/"), Some("/mnt/c".to_string()));
        assert_eq!(
            windows_to_wsl_path("/home/john/code/project"),
            Some("/home/john/code/project".to_string())
        );
    }

    #[test]
    fn test_wsl_to_windows() {
        assert_eq!(
            wsl_to_windows_path("/mnt/c/Users/john/AppData"),
            Some(r"C:\Users\john\AppData".to_string())
        );
        assert_eq!(
            wsl_to_windows_path("/mnt/d/code/project"),
            Some(r"D:\code\project".to_string())
        );
    }

    #[test]
    fn test_unc_path() {
        assert_eq!(
            windows_to_wsl_path(r"\\wsl$\Ubuntu\home\john"),
            Some("/home/john".to_string())
        );
        assert_eq!(
            windows_to_wsl_path(r"\\wsl.localhost\Ubuntu\home\john"),
            Some("/home/john".to_string())
        );
    }

    #[test]
    fn test_wsl_convert_error_display() {
        let err = WslConvertError::UnsupportedPlatform;
        assert_eq!(
            err.to_string(),
            "WSL path conversion is only available on Windows hosts"
        );

        let err = WslConvertError::WslpathFailed {
            exit_code: Some(1),
            stderr: "path not found".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "wslpath failed (exit code 1): path not found"
        );

        let err = WslConvertError::WslpathFailed {
            exit_code: None,
            stderr: "unknown error".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "wslpath failed (exit code unknown): unknown error"
        );

        let err = WslConvertError::InvalidOutput("empty".to_string());
        assert_eq!(err.to_string(), "wslpath returned invalid output: empty");
    }

    #[cfg(target_os = "windows")]
    fn with_env_var<T>(key: &str, value: Option<&str>, test: impl FnOnce() -> T) -> T {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().ok();
        let original = std::env::var(key).ok();
        match value {
            Some(next) => std::env::set_var(key, next),
            None => std::env::remove_var(key),
        }
        let result = test();
        match original {
            Some(previous) => std::env::set_var(key, previous),
            None => std::env::remove_var(key),
        }
        result
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_build_wslpath_command_args_uses_explicit_override() {
        with_env_var("INTERMEDIARY_WSL_DISTRO", Some("EnvDistro"), || {
            let args = build_wslpath_command_args("/home/john/code", Some("ConfiguredDistro"));
            assert_eq!(
                args,
                vec![
                    "-d".to_string(),
                    "ConfiguredDistro".to_string(),
                    "wslpath".to_string(),
                    "-w".to_string(),
                    "/home/john/code".to_string()
                ]
            );
        });
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_build_wslpath_command_args_trims_explicit_override() {
        with_env_var("INTERMEDIARY_WSL_DISTRO", None, || {
            let args = build_wslpath_command_args("/home/john/code", Some("  Ubuntu-22.04  "));
            assert_eq!(
                args,
                vec![
                    "-d".to_string(),
                    "Ubuntu-22.04".to_string(),
                    "wslpath".to_string(),
                    "-w".to_string(),
                    "/home/john/code".to_string()
                ]
            );
        });
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_build_wslpath_command_args_uses_env_fallback_when_override_missing() {
        with_env_var("INTERMEDIARY_WSL_DISTRO", Some("Ubuntu"), || {
            let args = build_wslpath_command_args("/home/john/code", None);
            assert_eq!(
                args,
                vec![
                    "-d".to_string(),
                    "Ubuntu".to_string(),
                    "wslpath".to_string(),
                    "-w".to_string(),
                    "/home/john/code".to_string()
                ]
            );
        });
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_build_wslpath_command_args_uses_env_fallback_when_override_blank() {
        with_env_var("INTERMEDIARY_WSL_DISTRO", Some("Ubuntu"), || {
            let args = build_wslpath_command_args("/home/john/code", Some("   "));
            assert_eq!(
                args,
                vec![
                    "-d".to_string(),
                    "Ubuntu".to_string(),
                    "wslpath".to_string(),
                    "-w".to_string(),
                    "/home/john/code".to_string()
                ]
            );
        });
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_build_wslpath_command_args_no_override_when_sources_missing() {
        with_env_var("INTERMEDIARY_WSL_DISTRO", None, || {
            let args = build_wslpath_command_args("/home/john/code", None);
            assert_eq!(
                args,
                vec![
                    "wslpath".to_string(),
                    "-w".to_string(),
                    "/home/john/code".to_string()
                ]
            );
        });
    }
}
