// Path: src-tauri/src/lib/paths/repo_root_resolver.rs
// Description: Path-native repo root resolver for user-selected repo paths

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoRootKind {
    Wsl,
    Host,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRepoRoot {
    pub kind: RepoRootKind,
    pub path: String,
}

/// Resolve an arbitrary user-provided path into a path-native repo root.
///
/// Rules:
/// - On Windows, `/mnt/<drive>/...` resolves to host Windows root.
/// - `\\wsl$\<distro>\mnt\<drive>\...` resolves to host Windows root.
/// - UNC WSL paths not under `/mnt/<drive>` resolve to WSL root.
/// - Absolute POSIX paths resolve to host root on non-Windows hosts.
/// - Drive-letter paths resolve to host root.
pub fn resolve_repo_root_from_input(input_path: &str) -> Option<ResolvedRepoRoot> {
    let trimmed = input_path.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with(r"\\wsl$\") || trimmed.starts_with(r"\\wsl.localhost\") {
        return resolve_unc_wsl_repo_root(trimmed);
    }

    if is_posix_absolute_path(trimmed) {
        return resolve_absolute_path_for_host(trimmed);
    }

    let host_path = normalize_host_windows_path(trimmed)?;
    Some(ResolvedRepoRoot {
        kind: RepoRootKind::Host,
        path: host_path,
    })
}

/// Resolve a legacy stored path from historical `wslPath`-centric configs.
///
/// Legacy rules are intentionally Windows-centric:
/// - `/mnt/<drive>/...` resolves to host Windows path.
/// - Other absolute POSIX paths resolve to WSL path.
/// - Drive-letter paths resolve to host Windows path.
pub fn resolve_legacy_repo_root_from_input(input_path: &str) -> Option<ResolvedRepoRoot> {
    let trimmed = input_path.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with(r"\\wsl$\") || trimmed.starts_with(r"\\wsl.localhost\") {
        return resolve_unc_wsl_repo_root(trimmed);
    }

    if is_posix_absolute_path(trimmed) {
        if let Some(host_path) = wsl_to_windows_path_for_repo_root(trimmed) {
            return Some(ResolvedRepoRoot {
                kind: RepoRootKind::Host,
                path: host_path,
            });
        }
        return Some(ResolvedRepoRoot {
            kind: RepoRootKind::Wsl,
            path: normalize_posix_path(trimmed),
        });
    }

    Some(ResolvedRepoRoot {
        kind: RepoRootKind::Host,
        path: normalize_host_windows_path(trimmed)?,
    })
}

fn resolve_absolute_path_for_host(path: &str) -> Option<ResolvedRepoRoot> {
    let normalized = normalize_posix_path(path);
    if cfg!(target_os = "windows") {
        if let Some(host_path) = wsl_to_windows_path_for_repo_root(&normalized) {
            return Some(ResolvedRepoRoot {
                kind: RepoRootKind::Host,
                path: host_path,
            });
        }
        return Some(ResolvedRepoRoot {
            kind: RepoRootKind::Wsl,
            path: normalized,
        });
    }

    Some(ResolvedRepoRoot {
        kind: RepoRootKind::Host,
        path: normalized,
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
                kind: RepoRootKind::Host,
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

fn normalize_host_windows_path(windows_path: &str) -> Option<String> {
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

fn normalize_posix_path(path: &str) -> String {
    let replaced = path.trim().replace('\\', "/");
    let segments: Vec<&str> = replaced
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.is_empty() {
        return "/".to_string();
    }
    format!("/{}", segments.join("/"))
}

fn is_posix_absolute_path(path: &str) -> bool {
    path.starts_with('/')
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
                kind: RepoRootKind::Host,
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
                kind: RepoRootKind::Host,
                path: r"C:\Users\john\repo".to_string()
            })
        );
        if cfg!(target_os = "windows") {
            assert_eq!(
                resolve_repo_root_from_input("/mnt/d/work/repo"),
                Some(ResolvedRepoRoot {
                    kind: RepoRootKind::Host,
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
        } else {
            assert_eq!(
                resolve_repo_root_from_input("/mnt/d/work/repo"),
                Some(ResolvedRepoRoot {
                    kind: RepoRootKind::Host,
                    path: "/mnt/d/work/repo".to_string()
                })
            );
            assert_eq!(
                resolve_repo_root_from_input("/home/john/repo"),
                Some(ResolvedRepoRoot {
                    kind: RepoRootKind::Host,
                    path: "/home/john/repo".to_string()
                })
            );
        }
    }

    #[test]
    fn test_resolve_legacy_repo_root_from_input() {
        assert_eq!(
            resolve_legacy_repo_root_from_input("/mnt/d/work/repo"),
            Some(ResolvedRepoRoot {
                kind: RepoRootKind::Host,
                path: r"D:\work\repo".to_string()
            })
        );
        assert_eq!(
            resolve_legacy_repo_root_from_input("/home/john/repo"),
            Some(ResolvedRepoRoot {
                kind: RepoRootKind::Wsl,
                path: "/home/john/repo".to_string()
            })
        );
    }
}
