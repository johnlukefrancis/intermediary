// Path: src-tauri/src/lib/paths/wsl_convert.rs
// Description: Windows <-> WSL path conversion utilities

/// Convert a Windows path to its WSL equivalent
///
/// Example: `C:\Users\john\AppData` -> `/mnt/c/Users/john/AppData`
pub fn windows_to_wsl_path(windows_path: &str) -> Option<String> {
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

    // Handle standard Windows paths like C:\...
    let mut chars = windows_path.chars();
    let drive_letter = chars.next()?.to_ascii_lowercase();

    // Verify it's a drive letter followed by :\
    if !drive_letter.is_ascii_alphabetic() {
        return None;
    }
    if chars.next() != Some(':') {
        return None;
    }
    if chars.next() != Some('\\') && !windows_path.ends_with(':') {
        return None;
    }

    let rest: String = chars.collect();
    let unix_rest = rest.replace('\\', "/");

    Some(format!("/mnt/{drive_letter}{unix_rest}"))
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

#[cfg(test)]
mod tests {
    use super::*;

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
    }
}
