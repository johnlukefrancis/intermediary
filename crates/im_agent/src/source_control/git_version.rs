// Path: crates/im_agent/src/source_control/git_version.rs
// Description: Once-per-process Git version probe guarding --pathspec-from-file support (Git 2.25+)

use std::ffi::OsString;
use std::path::Path;
use std::sync::OnceLock;

use im_bundle::git::run_git;

use crate::error::AgentError;

use super::runner::{git_executable, PROBE_LIMIT, PROBE_TIMEOUT};
use super::runner_failure::map_probe_failure;

/// `--pathspec-from-file` / `--pathspec-file-nul` arrived in Git 2.25.
const MINIMUM: (u32, u32) = (2, 25);

/// Cached only once the binary answered: a missing or broken Git is reported
/// every time so installing it later needs no agent restart.
static VERDICT: OnceLock<Verdict> = OnceLock::new();

#[derive(Debug, Clone)]
enum Verdict {
    Supported,
    Unsupported(String),
}

impl Verdict {
    fn to_result(&self) -> Result<(), AgentError> {
        match self {
            Verdict::Supported => Ok(()),
            Verdict::Unsupported(version) => Err(AgentError::new(
                "GIT_UNSUPPORTED_VERSION",
                format!(
                    "Git {version} is too old for source control; Git {}.{} or newer is required",
                    MINIMUM.0, MINIMUM.1
                ),
            )),
        }
    }
}

pub(super) fn ensure_supported(repo_root: &Path) -> Result<(), AgentError> {
    if let Some(verdict) = VERDICT.get() {
        return verdict.to_result();
    }
    let probed = probe(repo_root)?;
    VERDICT.get_or_init(|| probed).to_result()
}

fn probe(repo_root: &Path) -> Result<Verdict, AgentError> {
    let args = [OsString::from("--version")];
    let output = run_git(
        &git_executable(),
        repo_root,
        &args,
        PROBE_LIMIT,
        PROBE_TIMEOUT,
        None,
    )
    .map_err(|error| AgentError::internal(format!("Git version probe failed: {error}")))?
    .map_err(|failure| map_probe_failure(repo_root, failure))?;
    Ok(classify(String::from_utf8_lossy(&output.stdout).trim()))
}

/// `git version 2.53.0` or `git version 2.39.2.windows.1`. A line that does
/// not parse is not proven old and passes.
fn classify(version_line: &str) -> Verdict {
    match parse_version(version_line) {
        Some(version) if version < MINIMUM => {
            Verdict::Unsupported(format!("{}.{}", version.0, version.1))
        }
        _ => Verdict::Supported,
    }
}

fn parse_version(line: &str) -> Option<(u32, u32)> {
    let token = line
        .split_whitespace()
        .find(|token| token.starts_with(|c: char| c.is_ascii_digit()))?;
    let mut parts = token.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts
        .next()?
        .trim_end_matches(|c: char| !c.is_ascii_digit())
        .parse()
        .ok()?;
    Some((major, minor))
}

#[cfg(test)]
mod tests {
    use super::{classify, parse_version, Verdict};

    #[test]
    fn parses_common_version_lines() {
        assert_eq!(parse_version("git version 2.53.0"), Some((2, 53)));
        assert_eq!(parse_version("git version 2.39.2.windows.1"), Some((2, 39)));
        assert_eq!(parse_version("git version 2.25"), Some((2, 25)));
        assert_eq!(parse_version("git version"), None);
    }

    #[test]
    fn refuses_only_versions_proven_older_than_the_minimum() {
        assert!(matches!(classify("git version 2.24.3"), Verdict::Unsupported(v) if v == "2.24"));
        assert!(matches!(classify("git version 1.9.1"), Verdict::Unsupported(_)));
        assert!(matches!(classify("git version 2.25.0"), Verdict::Supported));
        assert!(matches!(classify("git version 3.0.0"), Verdict::Supported));
        assert!(matches!(classify("something odd"), Verdict::Supported));
    }
}
