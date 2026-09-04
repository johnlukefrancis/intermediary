// Path: crates/im_agent/src/staging/layout_unc.rs
// Description: Translation of Windows WSL UNC paths into this distro's own POSIX paths

//! `layout.rs` owns the `/mnt/<drive>` bridge, which is the *host* filesystem
//! seen from WSL. This file owns the other direction of the same boundary: a
//! Windows path that names the WSL filesystem itself, which Explorer and the
//! Tauri file dialog hand back as `\\wsl$\<distro>\…` or
//! `\\wsl.localhost\<distro>\…`.
//!
//! Only this distro's own share can be translated. `\\wsl$\Other\home\x` is a
//! different filesystem that this agent has no path to, and every other UNC
//! share (`\\server\share`, `\\?\C:\…`) is not a WSL namespace at all — all of
//! them answer `None` so the caller refuses the source rather than inventing a
//! local path that would silently name the wrong file.

const UNC_PREFIXES: [&str; 2] = ["\\\\wsl$\\", "\\\\wsl.localhost\\"];

/// This distro's POSIX path for a WSL UNC path, or `None` when the path is not
/// a UNC share this agent can speak for. The distro identity comes from
/// `WSL_DISTRO_NAME`, which WSL sets for every process it starts; without it
/// nothing is translated.
pub fn unc_to_wsl(unc_path: &str) -> Option<String> {
    let distro = std::env::var("WSL_DISTRO_NAME").ok()?;
    unc_to_wsl_for_distro(unc_path, &distro)
}

/// The translation itself, with the distro identity supplied. Split from the
/// environment read so it is testable without mutating process-wide state.
fn unc_to_wsl_for_distro(unc_path: &str, distro: &str) -> Option<String> {
    if distro.is_empty() {
        return None;
    }
    let normalized = unc_path.trim().replace('/', "\\");
    let rest = UNC_PREFIXES
        .iter()
        .find_map(|prefix| strip_prefix_ignore_ascii_case(&normalized, prefix))?;

    let mut parts = rest.splitn(2, '\\');
    let share = parts.next().unwrap_or_default();
    if !share.eq_ignore_ascii_case(distro) {
        return None;
    }

    let suffix = parts.next().unwrap_or("").trim_start_matches('\\');
    if suffix.is_empty() {
        return Some("/".to_string());
    }
    Some(format!("/{}", suffix.replace('\\', "/")))
}

fn strip_prefix_ignore_ascii_case<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    if value.len() < prefix.len() {
        return None;
    }
    let (head, tail) = value.split_at(prefix.len());
    head.eq_ignore_ascii_case(prefix).then_some(tail)
}

#[cfg(test)]
mod tests {
    use super::unc_to_wsl_for_distro;

    #[test]
    fn translates_both_unc_hosts_for_this_distro() {
        assert_eq!(
            unc_to_wsl_for_distro("\\\\wsl$\\Ubuntu\\home\\dev\\a.txt", "Ubuntu"),
            Some("/home/dev/a.txt".to_string())
        );
        assert_eq!(
            unc_to_wsl_for_distro("\\\\wsl.localhost\\Ubuntu\\home\\dev\\a.txt", "Ubuntu"),
            Some("/home/dev/a.txt".to_string())
        );
    }

    #[test]
    fn matches_the_distro_name_case_insensitively_and_accepts_slash_form() {
        assert_eq!(
            unc_to_wsl_for_distro("//wsl$/UBUNTU/home/dev", "ubuntu"),
            Some("/home/dev".to_string())
        );
    }

    #[test]
    fn the_share_root_is_the_filesystem_root() {
        assert_eq!(
            unc_to_wsl_for_distro("\\\\wsl$\\Ubuntu", "Ubuntu"),
            Some("/".to_string())
        );
        assert_eq!(
            unc_to_wsl_for_distro("\\\\wsl$\\Ubuntu\\", "Ubuntu"),
            Some("/".to_string())
        );
    }

    #[test]
    fn refuses_another_distro_and_every_non_wsl_share() {
        for path in [
            "\\\\wsl$\\Other\\home\\dev",
            "\\\\wsl.localhost\\Debian\\home\\dev",
            "\\\\server\\share\\file.txt",
            "\\\\?\\C:\\Users\\dev",
            "C:\\Users\\dev",
            "/home/dev",
            "",
        ] {
            assert_eq!(unc_to_wsl_for_distro(path, "Ubuntu"), None, "{path}");
        }
    }

    #[test]
    fn refuses_everything_when_the_distro_identity_is_empty() {
        assert_eq!(unc_to_wsl_for_distro("\\\\wsl$\\\\home\\dev", ""), None);
    }
}
