// Path: crates/im_agent/src/repos/import/translate.rs
// Description: Turning the OS paths a drop delivered into paths this agent's own namespace can reach

use std::path::PathBuf;

use crate::error::AgentError;
use crate::staging::{unc_to_wsl, windows_to_wsl, StagingRootKind};

use super::unsupported_source;

/// Every dropped path in this agent's own namespace.
///
/// Translation is a pure reading of the delivered string and owes nothing to
/// the filesystem, which is why it is a step of its own: an in-repo copy has
/// no host path form to interpret and enters the import at `plan_sources`
/// with paths that already live in this namespace.
pub(super) fn translate_sources(
    sources: &[String],
    staging_kind: StagingRootKind,
) -> Result<Vec<PathBuf>, AgentError> {
    sources
        .iter()
        .map(|raw| translate_source(raw, staging_kind))
        .collect()
}

/// The source path in this agent's own namespace. Host roots use the path as
/// delivered; a WSL root translates the Windows forms it can reach and refuses
/// the ones it cannot rather than guessing.
fn translate_source(raw: &str, staging_kind: StagingRootKind) -> Result<PathBuf, AgentError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(unsupported_source(raw, "is empty"));
    }

    match staging_kind {
        StagingRootKind::Host => {
            let path = PathBuf::from(trimmed);
            if !path.is_absolute() {
                return Err(unsupported_source(raw, "is not an absolute path"));
            }
            Ok(path)
        }
        StagingRootKind::Wsl => {
            if trimmed.starts_with("\\\\") {
                return unc_to_wsl(trimmed).map(PathBuf::from).ok_or_else(|| {
                    unsupported_source(raw, "names a share this agent's distro cannot reach")
                });
            }
            if let Some(translated) = unc_to_wsl(trimmed) {
                return Ok(PathBuf::from(translated));
            }
            if trimmed.starts_with('/') {
                return Ok(PathBuf::from(trimmed));
            }
            windows_to_wsl(trimmed)
                .map(PathBuf::from)
                .ok_or_else(|| unsupported_source(raw, "is not an absolute path"))
        }
    }
}
