// Path: src-tauri/src/lib/commands/wsl.rs
// Description: WSL host resolution for Windows->WSL agent connections

use std::process::Command;

/// Resolve the WSL host IP address (best-effort).
///
/// Returns the first IP from `hostname -I` in the configured WSL distro.
#[tauri::command]
pub fn resolve_wsl_host() -> Result<Option<String>, String> {
    let distro = std::env::var("INTERMEDIARY_WSL_DISTRO").unwrap_or_else(|_| "Ubuntu".to_string());

    let output = Command::new("wsl.exe")
        .args(["-d", &distro, "--", "hostname", "-I"])
        .output()
        .map_err(|e| format!("Failed to run wsl.exe: {e}"))?;

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let ip = stdout.split_whitespace().next().map(|value| value.to_string());
    Ok(ip)
}
