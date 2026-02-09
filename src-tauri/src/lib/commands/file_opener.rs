// Path: src-tauri/src/lib/commands/file_opener.rs
// Description: Reveal files in file manager or open with default application

use crate::config::types::RepoRoot;
use std::path::{Component, Path};
use std::process::Command;

use super::file_manager::resolve_host_path;

const TEXT_EXTENSIONS: &[&str] = &["txt", "md", "mdx", "rst", "adoc", "ts", "tsx", "js", "jsx", "mjs", "cjs", "json", "jsonc", "yaml", "yml", "toml", "ini", "cfg", "conf", "env", "rs", "py", "java", "kt", "kts", "go", "c", "h", "hpp", "hxx", "cc", "cpp", "cxx", "cs", "swift", "rb", "php", "sh", "bash", "zsh", "fish", "ps1", "bat", "cmd", "css", "scss", "less", "html", "htm", "xml", "svg", "sql", "vue", "svelte"];
const TEXT_BASENAMES: &[&str] = &["readme", "license", "makefile", "dockerfile", ".gitignore", ".gitattributes", ".npmignore"];
fn validate_relative_path(relative_path: &str) -> Result<(), String> {
    if relative_path.trim().is_empty() {
        return Err("Relative path cannot be empty".to_string());
    }

    let path = Path::new(relative_path);
    if path.is_absolute() {
        return Err("Absolute paths are not allowed".to_string());
    }

    let mut has_normal = false;
    for component in path.components() {
        match component {
            Component::ParentDir => {
                return Err("Path traversal is not allowed".to_string());
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err("Absolute paths are not allowed".to_string());
            }
            Component::Normal(_) => has_normal = true,
            Component::CurDir => {}
        }
    }

    if !has_normal {
        return Err("Relative path cannot be empty".to_string());
    }

    Ok(())
}
fn resolve_host_file_path(root: &RepoRoot, relative_path: &str) -> Result<String, String> {
    let normalized_relative = relative_path.trim().replace('\\', "/");
    validate_relative_path(&normalized_relative)?;

    let absolute_path = build_absolute_repo_path(root, &normalized_relative)?;
    resolve_host_path(&absolute_path)
}
fn resolve_host_file_paths(root: &RepoRoot, relative_paths: &[String]) -> Result<Vec<String>, String> {
    if relative_paths.is_empty() {
        return Err("No files provided".to_string());
    }

    let mut host_paths = Vec::with_capacity(relative_paths.len());
    for relative_path in relative_paths {
        let host_path = resolve_host_file_path(root, relative_path)?;
        let path = Path::new(&host_path);
        if !path.exists() || path.is_dir() {
            return Err(format!("File does not exist: {host_path}"));
        }
        host_paths.push(host_path);
    }

    Ok(host_paths)
}
fn is_text_relative_path(relative_path: &str) -> bool {
    let path = Path::new(relative_path);
    if let Some(ext) = path.extension().and_then(|value| value.to_str()) {
        let lower_ext = ext.to_ascii_lowercase();
        if TEXT_EXTENSIONS.contains(&lower_ext.as_str()) {
            return true;
        }
    }

    if let Some(name) = path.file_name().and_then(|value| value.to_str()) {
        let lower_name = name.to_ascii_lowercase();
        if TEXT_BASENAMES.contains(&lower_name.as_str()) {
            return true;
        }
    }

    false
}
fn open_paths_with_default_app(host_paths: &[String]) -> Result<(), String> {
    if host_paths.is_empty() {
        return Err("No files provided".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        for host_path in host_paths {
            Command::new("explorer")
                .arg(host_path)
                .spawn()
                .map_err(|e| format!("Failed to open file '{host_path}': {e}"))?;
        }
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(host_paths)
            .spawn()
            .map_err(|e| format!("Failed to open files: {e}"))?;
        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if Command::new("gio")
            .arg("open")
            .args(host_paths)
            .spawn()
            .is_ok()
        {
            return Ok(());
        }

        Command::new("sh")
            .arg("-c")
            .arg("for p in \"$@\"; do xdg-open \"$p\"; done")
            .arg("intermediary-open-files")
            .args(host_paths)
            .spawn()
            .map_err(|e| format!("Failed to open files: {e}"))?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err("open_file is not supported on this platform".to_string())
}
fn open_paths_with_native_text_editor(host_paths: &[String]) -> Result<(), String> {
    if host_paths.is_empty() {
        return Err("No files provided".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        for host_path in host_paths {
            Command::new("notepad.exe")
                .arg(host_path)
                .spawn()
                .map_err(|e| format!("Failed to open file '{host_path}' in Notepad: {e}"))?;
        }
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-a", "TextEdit"])
            .args(host_paths)
            .spawn()
            .map_err(|e| format!("Failed to open files in TextEdit: {e}"))?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err("native text editor is not supported on this platform".to_string())
}
fn open_paths_by_policy(relative_paths: &[String], host_paths: &[String]) -> Result<(), String> {
    if relative_paths.len() != host_paths.len() {
        return Err("Path mismatch while preparing file open".to_string());
    }

    let (mut text_paths, mut non_text_paths): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
    for (relative_path, host_path) in relative_paths.iter().zip(host_paths.iter()) {
        if is_text_relative_path(relative_path) {
            text_paths.push(host_path.clone());
        } else {
            non_text_paths.push(host_path.clone());
        }
    }

    let mut opened_any = false;
    let mut errors: Vec<String> = Vec::new();

    if !text_paths.is_empty() {
        match open_paths_with_native_text_editor(&text_paths) {
            Ok(()) => {
                opened_any = true;
            }
            Err(native_err) => match open_paths_with_default_app(&text_paths) {
                Ok(()) => {
                    opened_any = true;
                }
                Err(default_err) => {
                    errors.push(format!(
                        "Text-file open failed (native: {native_err}; default fallback: {default_err})"
                    ));
                }
            },
        }
    }

    if !non_text_paths.is_empty() {
        match open_paths_with_default_app(&non_text_paths) {
            Ok(()) => {
                opened_any = true;
            }
            Err(err) => {
                errors.push(format!("Non-text file open failed: {err}"));
            }
        }
    }

    if opened_any { Ok(()) } else if errors.is_empty() { Err("No files were opened".to_string()) } else { Err(errors.join("; ")) }
}
fn build_absolute_repo_path(root: &RepoRoot, normalized_relative: &str) -> Result<String, String> {
    let root_path = root.path().trim().to_string();
    if root_path.is_empty() {
        return Err("Repo root path cannot be empty".to_string());
    }

    match root {
        RepoRoot::Wsl { .. } => {
            let trimmed_root = root_path.trim_end_matches('/');
            if trimmed_root.is_empty() {
                return Ok(format!("/{}", normalized_relative));
            }
            Ok(format!("{trimmed_root}/{normalized_relative}"))
        }
        RepoRoot::Host { .. } => Ok(Path::new(&root_path)
            .join(normalized_relative)
            .to_string_lossy()
            .to_string()),
    }
}

#[tauri::command]
pub async fn reveal_in_file_manager(root: RepoRoot, relative_path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let host_path = resolve_host_file_path(&root, &relative_path)?;
        let path = Path::new(&host_path);
        if !path.exists() || path.is_dir() {
            return Err(format!("File does not exist: {host_path}"));
        }

        #[cfg(target_os = "windows")]
        {
            Command::new("explorer")
                .args(["/select,", &host_path])
                .spawn()
                .map_err(|e| format!("Failed to open Explorer: {e}"))?;
            return Ok::<(), String>(());
        }

        #[cfg(target_os = "macos")]
        {
            Command::new("open")
                .args(["-R", &host_path])
                .spawn()
                .map_err(|e| format!("Failed to reveal in Finder: {e}"))?;
            return Ok::<(), String>(());
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        {
            let parent = path
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| host_path.clone());
            Command::new("xdg-open")
                .arg(&parent)
                .spawn()
                .map_err(|e| format!("Failed to open file manager: {e}"))?;
            return Ok::<(), String>(());
        }

        #[allow(unreachable_code)]
        Err("reveal_in_file_manager is not supported on this platform".to_string())
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
}

#[tauri::command]
pub async fn open_file(root: RepoRoot, relative_path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let relative_paths = vec![relative_path];
        open_paths_by_policy(&relative_paths, &resolve_host_file_paths(&root, &relative_paths)?)
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
}

#[tauri::command]
pub async fn open_files(root: RepoRoot, relative_paths: Vec<String>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        open_paths_by_policy(&relative_paths, &resolve_host_file_paths(&root, &relative_paths)?)
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
}
