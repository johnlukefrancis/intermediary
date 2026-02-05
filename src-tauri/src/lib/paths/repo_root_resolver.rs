// Path: src-tauri/src/lib/paths/repo_root_resolver.rs
// Description: Path-native repo root resolver for user-selected repo paths

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoRootKind {
    Wsl,
    Windows,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRepoRoot {
    pub kind: RepoRootKind,
    pub path: String,
}

/// Resolve an arbitrary user-provided path into a path-native repo root.
///
/// Rules:
/// - `/mnt/<drive>/...` resolves to Windows root.
/// - `\\wsl$\<distro>\mnt\<drive>\...` resolves to Windows root.
/// - Other absolute Linux/UNC WSL paths resolve to WSL root.
/// - Drive-letter paths resolve to Windows root.
pub fn resolve_repo_root_from_input(input_path: &str) -> Option<ResolvedRepoRoot> {
    let trimmed = input_path.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with(r"\\wsl$\") || trimmed.starts_with(r"\\wsl.localhost\") {
        return resolve_unc_wsl_repo_root(trimmed);
    }

    if trimmed.starts_with('/') {
        if let Some(windows_path) = wsl_to_windows_path_for_repo_root(trimmed) {
            return Some(ResolvedRepoRoot {
                kind: RepoRootKind::Windows,
                path: windows_path,
            });
        }
        return Some(ResolvedRepoRoot {
            kind: RepoRootKind::Wsl,
            path: trimmed.replace('\\', "/"),
        });
    }

    let windows_path = normalize_windows_path(trimmed)?;
    Some(ResolvedRepoRoot {
        kind: RepoRootKind::Windows,
        path: windows_path,
    })
}

fn resolve_unc_wsl_repo_root(input_path: &str) -> Option<ResolvedRepoRoot> {
    let rest = input_path
        .strip_prefix(r"\\wsl$\")
        .or_else(|| input_path.strip_prefix(r"\\wsl.localhost\"))?;

    let segments: Vec<&str> = rest
        .split(['\\', '/'])
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.is_empty() {
        return None;
    }

    let path_segments = &segments[1..]; // Drop distro name.
    if path_segments.is_empty() {
        return Some(ResolvedRepoRoot {
            kind: RepoRootKind::Wsl,
            path: "/".to_string(),
        });
    }

    if path_segments.len() >= 2
        && path_segments[0].eq_ignore_ascii_case("mnt")
        && path_segments[1].len() == 1
    {
        let drive = path_segments[1].chars().next()?.to_ascii_uppercase();
        if drive.is_ascii_alphabetic() {
            let rest = path_segments[2..].join("\\");
            let path = if rest.is_empty() {
                format!("{drive}:\\")
            } else {
                format!("{drive}:\\{rest}")
            };
            return Some(ResolvedRepoRoot {
                kind: RepoRootKind::Windows,
                path,
            });
        }
    }

    let path = format!("/{}", path_segments.join("/"));
    Some(ResolvedRepoRoot {
        kind: RepoRootKind::Wsl,
        path,
    })
}

fn normalize_windows_path(windows_path: &str) -> Option<String> {
    let normalized = windows_path.trim().replace('/', "\\");
    let mut chars = normalized.chars();
    let drive_letter = chars.next()?.to_ascii_uppercase();

    if !drive_letter.is_ascii_alphabetic() {
        return None;
    }
    if chars.next() != Some(':') {
        return None;
    }

    let rest: String = chars.collect();
    if rest.is_empty() {
        return Some(format!("{drive_letter}:\\"));
    }

    let trimmed = rest.trim_start_matches(['\\', '/'].as_slice());
    if trimmed.is_empty() {
        return Some(format!("{drive_letter}:\\"));
    }

    Some(format!("{drive_letter}:\\{}", trimmed.replace('/', "\\")))
}

fn wsl_to_windows_path_for_repo_root(wsl_path: &str) -> Option<String> {
    let stripped = wsl_path.strip_prefix("/mnt/")?;
    let mut chars = stripped.chars();
    let drive_letter = chars.next()?.to_ascii_uppercase();

    if !drive_letter.is_ascii_alphabetic() {
        return None;
    }

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
    fn test_resolve_repo_root_from_input() {
        assert_eq!(
            resolve_repo_root_from_input(r"C:\Users\john\code\repo"),
            Some(ResolvedRepoRoot {
                kind: RepoRootKind::Windows,
                path: r"C:\Users\john\code\repo".to_string()
            })
        );
        assert_eq!(
            resolve_repo_root_from_input(r"\\wsl$\Ubuntu\home\john\repo"),
            Some(ResolvedRepoRoot {
                kind: RepoRootKind::Wsl,
                path: "/home/john/repo".to_string()
            })
        );
        assert_eq!(
            resolve_repo_root_from_input(r"\\wsl$\Ubuntu\mnt\c\Users\john\repo"),
            Some(ResolvedRepoRoot {
                kind: RepoRootKind::Windows,
                path: r"C:\Users\john\repo".to_string()
            })
        );
        assert_eq!(
            resolve_repo_root_from_input("/mnt/d/work/repo"),
            Some(ResolvedRepoRoot {
                kind: RepoRootKind::Windows,
                path: r"D:\work\repo".to_string()
            })
        );
        assert_eq!(
            resolve_repo_root_from_input("/home/john/repo"),
            Some(ResolvedRepoRoot {
                kind: RepoRootKind::Wsl,
                path: "/home/john/repo".to_string()
            })
        );
    }
}
