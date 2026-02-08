// Path: crates/im_agent/src/bundles/bundle_lister.rs
// Description: Bundle listing and latest selection logic

use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde_json::Value;
use tokio::fs;

use crate::error::AgentError;
use crate::protocol::BundleInfo;
use crate::staging::{PathBridgeConfig, StagingLayout, StagingRootKind};

const MANIFEST_NAME: &str = "BUNDLE_MANIFEST.json";

pub struct ListBundlesOptions {
    pub staging: PathBridgeConfig,
    pub staging_kind: StagingRootKind,
    pub repo_id: String,
    pub preset_id: String,
}

pub async fn list_bundles(options: ListBundlesOptions) -> Result<Vec<BundleInfo>, AgentError> {
    let layout = StagingLayout::from_config(&options.staging, options.staging_kind)?;
    let target_dir = layout.bundles_dir(&options.repo_id, &options.preset_id);

    let mut entries = match fs::read_dir(&target_dir).await {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => {
            return Err(AgentError::internal(format!(
                "Failed to read bundles dir: {err}"
            )))
        }
    };

    let prefix = format!("{}_{}_", options.repo_id, options.preset_id);
    let mut candidates = Vec::new();

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|err| AgentError::internal(format!("Failed to read bundles dir: {err}")))?
    {
        let file_type = entry
            .file_type()
            .await
            .map_err(|err| AgentError::internal(format!("Failed to read bundle entry: {err}")))?;
        if !file_type.is_file() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().to_string();
        if !file_name.starts_with(&prefix) || !file_name.ends_with(".zip") {
            continue;
        }

        let local_path = entry.path();
        let metadata = entry
            .metadata()
            .await
            .map_err(|err| AgentError::internal(format!("Failed to stat bundle: {err}")))?;

        let mtime_ms = metadata
            .modified()
            .ok()
            .and_then(|mtime| mtime.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0);

        let timestamp = parse_timestamp_from_name(&file_name, &prefix);

        candidates.push(BundleCandidate {
            file_name,
            local_path,
            bytes: metadata.len(),
            mtime_ms,
            timestamp,
            manifest_time: None,
        });
    }

    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    for candidate in &mut candidates {
        if candidate.timestamp.is_none() {
            candidate.manifest_time = read_manifest_time(&candidate.local_path).await;
        }
    }

    candidates.sort_by(|a, b| {
        let a_time = resolve_sort_time(a);
        let b_time = resolve_sort_time(b);
        b_time
            .cmp(&a_time)
            .then_with(|| a.file_name.cmp(&b.file_name))
    });

    let mut results = Vec::with_capacity(candidates.len());
    for (index, candidate) in candidates.into_iter().enumerate() {
        let views = layout.path_views_for_runtime_path(&candidate.local_path)?;
        results.push(BundleInfo {
            host_path: views.host_path.clone(),
            legacy_windows_path: Some(views.host_path),
            file_name: candidate.file_name,
            bytes: candidate.bytes,
            mtime_ms: candidate.mtime_ms,
            is_latest_alias: index == 0,
        });
    }

    Ok(results)
}

struct BundleCandidate {
    file_name: String,
    local_path: PathBuf,
    bytes: u64,
    mtime_ms: u64,
    timestamp: Option<DateTime<Utc>>,
    manifest_time: Option<DateTime<Utc>>,
}

fn resolve_sort_time(candidate: &BundleCandidate) -> DateTime<Utc> {
    candidate
        .timestamp
        .or(candidate.manifest_time)
        .or_else(|| mtime_to_datetime(candidate.mtime_ms))
        .unwrap_or_else(|| DateTime::<Utc>::from(std::time::UNIX_EPOCH))
}

fn mtime_to_datetime(mtime_ms: u64) -> Option<DateTime<Utc>> {
    let millis = i64::try_from(mtime_ms).ok()?;
    Utc.timestamp_millis_opt(millis).single()
}

fn parse_timestamp_from_name(file_name: &str, prefix: &str) -> Option<DateTime<Utc>> {
    if !file_name.starts_with(prefix) || !file_name.ends_with(".zip") {
        return None;
    }

    let trimmed = &file_name[prefix.len()..file_name.len() - 4];
    let ts = trimmed.get(0..15)?;
    let naive = NaiveDateTime::parse_from_str(ts, "%Y%m%d_%H%M%S").ok()?;
    Some(Utc.from_utc_datetime(&naive))
}

async fn read_manifest_time(path: &Path) -> Option<DateTime<Utc>> {
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || read_manifest_time_blocking(&path))
        .await
        .ok()
        .flatten()
}

fn read_manifest_time_blocking(path: &Path) -> Option<DateTime<Utc>> {
    let file = std::fs::File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let mut manifest = archive.by_name(MANIFEST_NAME).ok()?;
    let mut contents = String::new();
    use std::io::Read;
    manifest.read_to_string(&mut contents).ok()?;
    let parsed: Value = serde_json::from_str(&contents).ok()?;
    let generated_at = parsed.get("generatedAt")?.as_str()?;
    DateTime::parse_from_rfc3339(generated_at)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn selects_latest_by_timestamp() {
        let base = Path::new("/mnt/c");
        if !base.exists() {
            return;
        }

        let root = TempDir::new_in(base).expect("tempdir");
        let repo_id = "repo";
        let preset_id = "preset";
        let dir = root.path().join("bundles").join(repo_id).join(preset_id);
        fs::create_dir_all(&dir).await.expect("mkdir");

        let names = vec![
            "repo_preset_20240101_010101.zip",
            "repo_preset_20240102_010101.zip",
            "repo_preset_20231231_235959.zip",
        ];

        for name in &names {
            let path = dir.join(name);
            let mut file = fs::File::create(&path).await.expect("create");
            file.write_all(b"test").await.expect("write");
        }

        let results = list_bundles(ListBundlesOptions {
            staging: PathBridgeConfig {
                staging_host_root: "C:\\bundles".to_string(),
                staging_wsl_root: Some(root.path().to_string_lossy().to_string()),
            },
            staging_kind: StagingRootKind::Wsl,
            repo_id: repo_id.to_string(),
            preset_id: preset_id.to_string(),
        })
        .await
        .expect("list bundles");

        assert_eq!(results[0].file_name, "repo_preset_20240102_010101.zip");
        assert!(results[0].is_latest_alias);
    }
}
