// Path: src-tauri/src/lib/commands/file_open_policy.rs
// Description: Host launcher policy for opening text and non-text files

use std::path::Path;
use std::process::Command;

const TEXT_EXTENSIONS: &[&str] = &[
    "txt", "md", "mdx", "rst", "adoc", "ts", "tsx", "js", "jsx", "mjs", "cjs", "json", "jsonc",
    "yaml", "yml", "toml", "ini", "cfg", "conf", "env", "rs", "py", "java", "kt", "kts", "go", "c",
    "h", "hpp", "hxx", "cc", "cpp", "cxx", "cs", "swift", "rb", "php", "sh", "bash", "zsh", "fish",
    "ps1", "bat", "cmd", "css", "scss", "less", "html", "htm", "xml", "svg", "sql", "vue",
    "svelte",
];
const TEXT_BASENAMES: &[&str] = &[
    "readme",
    "license",
    "makefile",
    "dockerfile",
    "gemfile",
    "podfile",
    "rakefile",
    "brewfile",
    "justfile",
    ".gitignore",
    ".gitattributes",
    ".npmignore",
];

pub(crate) fn open_paths_by_policy(
    relative_paths: &[String],
    host_paths: &[String],
) -> Result<(), String> {
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

    if opened_any {
        Ok(())
    } else if errors.is_empty() {
        Err("No files were opened".to_string())
    } else {
        Err(errors.join("; "))
    }
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
