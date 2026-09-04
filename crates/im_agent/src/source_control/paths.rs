// Path: crates/im_agent/src/source_control/paths.rs
// Description: UI path validation and normalization, NUL-joined pathspec input, and the in-root containment guard

use std::io;
use std::path::{Component, Path};

use crate::error::AgentError;
use crate::staging::validate_relative_path;

/// Git flags that read NUL-separated pathspecs from stdin, so no path list
/// ever meets an argv ceiling and no shell or glob interprets it.
pub(super) const PATHSPEC_FROM_STDIN: [&str; 2] = ["--pathspec-from-file=-", "--pathspec-file-nul"];

/// Validates one UI-supplied repo-relative path and normalizes it to the
/// slash-joined form Git prints (`./a//b/` becomes `a/b`) so classification
/// against `ls-files` output matches byte for byte.
pub(crate) fn normalize_path(path: &str) -> Result<String, AgentError> {
    validate_relative_path(path)?;
    if path.contains('\0') {
        return Err(AgentError::new(
            "INVALID_PATH",
            "NUL bytes not allowed in relative paths",
        ));
    }
    let parts: Vec<&str> = Path::new(path)
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => part.to_str(),
            _ => None,
        })
        .collect();
    Ok(parts.join("/"))
}

pub(super) fn normalize_paths(paths: &[String]) -> Result<Vec<String>, AgentError> {
    paths.iter().map(|path| normalize_path(path)).collect()
}

pub(super) fn nul_joined(paths: &[String]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(paths.iter().map(|path| path.len() + 1).sum());
    for path in paths {
        bytes.extend_from_slice(path.as_bytes());
        bytes.push(0);
    }
    bytes
}

/// Refuses a `.git` component anywhere in an already-normalized repo-relative
/// path. No write that bypasses Git may reach the repository's own directory,
/// at any depth, on either side of the operation. Compared without case
/// because the filesystems this agent writes (NTFS, drvfs) reach the same
/// directory through `.GIT`, and a check a filesystem can walk around is not a
/// check.
pub(crate) fn ensure_no_git_component(path: &str) -> Result<(), AgentError> {
    if path.split('/').any(|part| part.eq_ignore_ascii_case(".git")) {
        return Err(AgentError::new(
            "INVALID_PATH",
            format!("Refusing {path}: it names the repository's own Git directory"),
        ));
    }
    Ok(())
}

/// Confirms `path`'s parent directory resolves inside the repo root before a
/// caller touches it directly, bypassing Git: a discard claim renaming a file
/// away, or an import writing one in. A symlinked directory component must
/// never let a relative, traversal-free path still reach outside the
/// worktree. A path whose parent does not exist yet is not this guard's
/// concern — nothing can be reached through a directory that is not there,
/// and the caller creates it as a real directory or fails on the missing file.
pub(crate) fn ensure_within_root(repo_root: &Path, path: &str) -> Result<(), AgentError> {
    let canonical_root = repo_root.canonicalize().map_err(|error| {
        AgentError::new(
            "INVALID_REPO",
            format!("Repo root is not reachable: {error}"),
        )
    })?;
    let target = repo_root.join(path);
    let parent = target.parent().unwrap_or(repo_root);
    let canonical_parent = match parent.canonicalize() {
        Ok(resolved) => resolved,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(AgentError::internal(format!(
                "Failed to resolve {}: {error}",
                target.display()
            )))
        }
    };
    if !canonical_parent.starts_with(&canonical_root) {
        return Err(AgentError::new(
            "INVALID_PATH",
            format!("Refusing {path}: it resolves outside the repo root"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ensure_no_git_component, normalize_path, nul_joined};

    #[test]
    fn normalizes_to_git_printed_form() {
        assert_eq!(normalize_path("a.txt").expect("plain"), "a.txt");
        assert_eq!(normalize_path("./sub//a.txt").expect("dotted"), "sub/a.txt");
        assert_eq!(normalize_path("sub/dir/").expect("trailing"), "sub/dir");
        assert_eq!(normalize_path("a[1].txt").expect("glob chars"), "a[1].txt");
        assert_eq!(normalize_path(":colon.txt").expect("magic prefix"), ":colon.txt");
    }

    #[test]
    fn rejects_traversal_absolute_and_nul() {
        for bad in ["../x", "/abs", "", ".", "a\\b", "a\0b"] {
            let error = normalize_path(bad).expect_err(bad);
            assert_eq!(error.code(), "INVALID_PATH", "{bad}");
        }
    }

    #[test]
    fn the_git_directory_is_refused_at_any_depth_and_only_as_a_whole_component() {
        for bad in [".git", "app/.git", "a/.GIT/config"] {
            assert_eq!(
                ensure_no_git_component(bad).expect_err(bad).code(),
                "INVALID_PATH",
                "{bad}"
            );
        }
        ensure_no_git_component("app/.gitignore").expect("not a component");
        ensure_no_git_component("app/notes.git.txt").expect("not a component");
    }

    #[test]
    fn joins_with_trailing_nul_per_path() {
        assert_eq!(nul_joined(&["a".to_string(), "b c".to_string()]), b"a\0b c\0");
        assert!(nul_joined(&[]).is_empty());
    }
}
