// Path: src-tauri/src/lib/launch/target.rs
// Description: Resolve supported project launch targets from explicit URLs, host files or top-level repo files

use std::path::Path;

use super::{kind_for_file_name, LaunchKind};

pub(crate) fn web_url(target: &str) -> bool {
    tauri::Url::parse(target)
        .is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
}

pub(crate) fn standalone_target(target: &str) -> bool {
    Path::new(target).is_absolute() || web_url(target)
}

pub(crate) fn resolve_target(
    host_root: &str,
    target: &str,
) -> Result<(String, LaunchKind), String> {
    if web_url(target) {
        return Ok((target.to_string(), LaunchKind::Web));
    }
    let kind = kind_for_file_name(target)
        .ok_or_else(|| format!("'{target}' is not a supported project launch target"))?;
    let path = if Path::new(target).is_absolute() {
        target.to_string()
    } else {
        if target.contains(['/', '\\', ':']) || target.is_empty() {
            return Err(format!(
                "'{target}' is not a top-level repo file or absolute host file"
            ));
        }
        Path::new(host_root)
            .join(target)
            .to_string_lossy()
            .to_string()
    };
    if !Path::new(&path).is_file() {
        return Err(format!("Launch target does not exist: {path}"));
    }
    Ok((path, kind))
}

#[cfg(test)]
mod tests {
    use super::resolve_target;
    use crate::launch::LaunchKind;

    #[test]
    fn explicit_gallery_is_independent_of_repo_discovery() {
        let repo = tempfile::tempdir().unwrap();
        let gallery = tempfile::tempdir().unwrap();
        let page = gallery.path().join("gallery.html");
        std::fs::write(&page, "<html></html>").unwrap();
        let root = repo.path().to_str().unwrap();
        assert_eq!(
            resolve_target(root, page.to_str().unwrap()),
            Ok((page.to_string_lossy().to_string(), LaunchKind::Html))
        );
        assert_eq!(
            resolve_target(root, "http://localhost:8788/repairs/"),
            Ok((
                "http://localhost:8788/repairs/".to_string(),
                LaunchKind::Web
            ))
        );
        assert!(resolve_target(root, "missing.html").is_err());
        assert!(resolve_target(root, "../gallery.html").is_err());
        assert!(resolve_target(root, "javascript:alert(1)").is_err());
        assert!(resolve_target(root, "file:///J:/gallery.html").is_err());
    }
}
