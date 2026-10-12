// Path: src-tauri/src/lib/launch/unreal_engine.rs
// Description: Resolves a .uproject's EngineAssociation to the UnrealEditor.exe that should open it

use std::path::{Path, PathBuf};

const EDITOR_RELATIVE: &str = r"Engine\Binaries\Win64\UnrealEditor.exe";

/// The editor binary for `project`, from custom builds, launcher registry keys or the launcher's install list.
pub fn resolve_editor(project: &Path) -> Result<PathBuf, String> {
    let association = engine_association(project)?;
    let engine_root = engine_root(&association).ok_or_else(|| {
        format!("Unreal engine '{association}' is not registered on this machine")
    })?;
    let editor = Path::new(&engine_root).join(EDITOR_RELATIVE);
    if editor.is_file() {
        Ok(editor)
    } else {
        Err(format!(
            "Unreal engine '{association}' has no editor at {}",
            editor.display()
        ))
    }
}

fn engine_association(project: &Path) -> Result<String, String> {
    let text = std::fs::read_to_string(project)
        .map_err(|e| format!("Failed to read {}: {e}", project.display()))?;
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("{} is not valid JSON: {e}", project.display()))?;
    json.get("EngineAssociation")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| format!("{} names no EngineAssociation", project.display()))
}

#[cfg(windows)]
fn engine_root(association: &str) -> Option<String> {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    registry_string(HKEY_CURRENT_USER, r"Software\Epic Games\Unreal Engine\Builds", association)
        .or_else(|| {
            let subkey = format!(r"SOFTWARE\EpicGames\Unreal Engine\{association}");
            registry_string(HKEY_LOCAL_MACHINE, &subkey, "InstalledDirectory")
        })
        .or_else(|| launcher_install_location(association))
}

#[cfg(not(windows))]
fn engine_root(_association: &str) -> Option<String> {
    None
}

#[cfg(windows)]
fn launcher_install_location(association: &str) -> Option<String> {
    let program_data = std::env::var_os("ProgramData")?;
    let list = Path::new(&program_data).join(r"Epic\UnrealEngineLauncher\LauncherInstalled.dat");
    let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(list).ok()?).ok()?;
    let app_name = format!("UE_{association}");
    json.get("InstallationList")?
        .as_array()?
        .iter()
        .find(|item| item.get("AppName").and_then(|v| v.as_str()) == Some(app_name.as_str()))?
        .get("InstallLocation")?
        .as_str()
        .map(ToString::to_string)
}

#[cfg(windows)]
fn registry_string(
    root: windows_sys::Win32::System::Registry::HKEY,
    subkey: &str,
    value: &str,
) -> Option<String> {
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{RegGetValueW, RRF_RT_REG_SZ};

    let subkey = wide(subkey);
    let value = wide(value);
    let mut buffer = vec![0u16; 1024];
    let mut size_bytes = (buffer.len() * std::mem::size_of::<u16>()) as u32;
    // SAFETY: the name buffers are NUL-terminated and live across the call; `size_bytes`
    // carries the output buffer's byte capacity in and the bytes written out.
    let status = unsafe {
        RegGetValueW(
            root,
            subkey.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut size_bytes,
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    let units = (size_bytes as usize / std::mem::size_of::<u16>()).min(buffer.len());
    let text = String::from_utf16_lossy(&buffer[..units]);
    let text = text.trim_end_matches('\0').trim();
    (!text.is_empty()).then(|| text.to_string())
}

#[cfg(windows)]
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
