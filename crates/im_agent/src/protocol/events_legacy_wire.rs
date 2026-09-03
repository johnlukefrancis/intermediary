// Path: crates/im_agent/src/protocol/events_legacy_wire.rs
// Description: Legacy hostPath/windowsPath wire shapes and conversions for staged-info and bundle-built events

use serde::Deserialize;

use super::events::{BundleBuiltEvent, StagedInfo};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct StagedInfoWire {
    #[serde(default)]
    host_path: Option<String>,
    #[serde(default)]
    windows_path: Option<String>,
    #[serde(default)]
    wsl_path: Option<String>,
    bytes_copied: u64,
    mtime_ms: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BundleBuiltEventWire {
    repo_id: String,
    preset_id: String,
    #[serde(default)]
    host_path: Option<String>,
    #[serde(default)]
    windows_path: Option<String>,
    #[serde(default)]
    alias_host_path: Option<String>,
    #[serde(default)]
    alias_windows_path: Option<String>,
    bytes: u64,
    file_count: u64,
    built_at_iso: String,
}

impl TryFrom<StagedInfoWire> for StagedInfo {
    type Error = String;

    fn try_from(value: StagedInfoWire) -> Result<Self, Self::Error> {
        let (host_path, legacy_windows_path) = resolve_required_path_pair(
            value.host_path,
            value.windows_path,
            "hostPath",
            "windowsPath",
        )?;

        Ok(Self {
            host_path,
            legacy_windows_path,
            wsl_path: value.wsl_path,
            bytes_copied: value.bytes_copied,
            mtime_ms: value.mtime_ms,
        })
    }
}

impl TryFrom<BundleBuiltEventWire> for BundleBuiltEvent {
    type Error = String;

    fn try_from(value: BundleBuiltEventWire) -> Result<Self, Self::Error> {
        let (host_path, legacy_windows_path) = resolve_required_path_pair(
            value.host_path,
            value.windows_path,
            "hostPath",
            "windowsPath",
        )?;
        let (alias_host_path, legacy_alias_windows_path) = resolve_required_path_pair(
            value.alias_host_path,
            value.alias_windows_path,
            "aliasHostPath",
            "aliasWindowsPath",
        )?;

        Ok(Self {
            repo_id: value.repo_id,
            preset_id: value.preset_id,
            host_path,
            legacy_windows_path,
            alias_host_path,
            legacy_alias_windows_path,
            bytes: value.bytes,
            file_count: value.file_count,
            built_at_iso: value.built_at_iso,
        })
    }
}

fn resolve_required_path_pair(
    canonical: Option<String>,
    legacy: Option<String>,
    canonical_name: &str,
    legacy_name: &str,
) -> Result<(String, Option<String>), String> {
    match (canonical, legacy) {
        (Some(canonical), Some(legacy)) => {
            if canonical != legacy {
                return Err(format!("conflicting {canonical_name}/{legacy_name} values"));
            }
            Ok((canonical, Some(legacy)))
        }
        (Some(canonical), None) => Ok((canonical, None)),
        (None, Some(legacy)) => Ok((legacy.clone(), Some(legacy))),
        (None, None) => Err(format!("missing {canonical_name}")),
    }
}
