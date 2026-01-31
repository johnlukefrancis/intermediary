// Path: crates/im_bundle/src/writer.rs
// Description: Bundle zip writer with scanning, manifest, and progress

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::time::Instant;

use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::error::{BundleError, Result};
use crate::manifest::build_manifest;
use crate::plan::BundlePlan;
use crate::progress::ProgressEmitter;
use crate::scanner::{scan_bundle, ScanEntry};

const BUFFER_SIZE: usize = 64 * 1024;
const MANIFEST_NAME: &str = "INTERMEDIARY_MANIFEST.json";
const COMPRESSION_LEVEL: i64 = 6;

#[derive(Debug)]
pub struct BundleResult {
    pub bytes_written: u64,
    pub file_count: u64,
    pub scan_ms: u128,
    pub zip_ms: u128,
}

pub fn write_bundle(plan: &BundlePlan) -> Result<BundleResult> {
    let mut progress = ProgressEmitter::new();

    let scan_start = Instant::now();
    let scan_result = scan_bundle(plan, &mut progress)?;
    let scan_ms = scan_start.elapsed().as_millis();

    let total_files = scan_result.entries.len() as u64 + 1;
    progress.emit_progress("zipping", 0, total_files);

    let zip_start = Instant::now();
    let (bytes_written, file_count) = write_zip(plan, &scan_result.entries, &scan_result.top_level_dirs_included, &mut progress)?;
    let zip_ms = zip_start.elapsed().as_millis();

    progress.emit_done(bytes_written, file_count, scan_ms, zip_ms);

    Ok(BundleResult {
        bytes_written,
        file_count,
        scan_ms,
        zip_ms,
    })
}

fn write_zip(
    plan: &BundlePlan,
    entries: &[ScanEntry],
    top_level_dirs_included: &[String],
    progress: &mut ProgressEmitter,
) -> Result<(u64, u64)> {
    let output_file = File::create(&plan.output_path).map_err(|source| {
        BundleError::OutputCreateFailed {
            path: plan.output_path.clone(),
            source,
        }
    })?;

    let writer = BufWriter::new(output_file);
    let mut zip = zip::ZipWriter::new(writer);

    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(COMPRESSION_LEVEL));

    let mut bytes_copied = 0u64;
    let mut files_done = 0u64;
    let mut buffer = vec![0u8; BUFFER_SIZE];

    for entry in entries {
        bytes_copied += write_entry(&mut zip, entry, options, &mut buffer)?;
        files_done += 1;
        progress.emit_progress("zipping", files_done, entries.len() as u64 + 1);
    }

    let (manifest_json, total_bytes_best_effort) = build_manifest_json(
        plan,
        top_level_dirs_included,
        bytes_copied,
        entries.len() as u64 + 1,
    )?;

    zip.start_file(MANIFEST_NAME, options)?;
    zip.write_all(manifest_json.as_bytes())
        .map_err(|e| BundleError::FinalizeFailed(format!("failed to write manifest: {e}")))?;
    files_done += 1;
    progress.emit_progress("zipping", files_done, entries.len() as u64 + 1);

    let writer = zip.finish()?;
    let file = writer
        .into_inner()
        .map_err(|e| BundleError::FinalizeFailed(format!("failed to flush buffer: {e}")))?;

    file.sync_all()
        .map_err(|e| BundleError::FinalizeFailed(format!("failed to sync file: {e}")))?;

    let bytes_written = file.metadata().map(|m| m.len()).unwrap_or(0);

    if total_bytes_best_effort != bytes_copied + manifest_json.len() as u64 {
        // Keep calculation local; manifest already written.
    }

    Ok((bytes_written, files_done))
}

fn write_entry(
    zip: &mut zip::ZipWriter<BufWriter<File>>,
    entry: &ScanEntry,
    options: SimpleFileOptions,
    buffer: &mut [u8],
) -> Result<u64> {
    let source_file = File::open(&entry.source_path).map_err(|source| BundleError::FileOpenFailed {
        path: entry.source_path.clone(),
        source,
    })?;
    let mut reader = BufReader::new(source_file);

    zip.start_file(&entry.archive_path, options)?;

    let mut total = 0u64;
    loop {
        let bytes_read = reader.read(buffer).map_err(|source| BundleError::FileReadFailed {
            path: entry.source_path.clone(),
            source,
        })?;
        if bytes_read == 0 {
            break;
        }
        zip.write_all(&buffer[..bytes_read])
            .map_err(|e| BundleError::FinalizeFailed(format!("failed to write to archive: {e}")))?;
        total += bytes_read as u64;
    }

    Ok(total)
}

fn build_manifest_json(
    plan: &BundlePlan,
    top_level_dirs_included: &[String],
    bytes_copied: u64,
    file_count: u64,
) -> Result<(String, u64)> {
    let mut total_bytes = bytes_copied;
    let mut manifest_json = String::new();

    for _ in 0..3 {
        let manifest = build_manifest(
            &plan.built_at_iso,
            &plan.repo_id,
            &plan.repo_root.to_string_lossy(),
            &plan.preset_id,
            &plan.preset_name,
            &plan.selection,
            top_level_dirs_included,
            &plan.git,
            file_count,
            total_bytes,
        );
        let json = serde_json::to_string(&manifest)?;
        let manifest_bytes = json.as_bytes().len() as u64;
        let new_total = bytes_copied + manifest_bytes;
        manifest_json = json;
        if new_total == total_bytes {
            return Ok((manifest_json, total_bytes));
        }
        total_bytes = new_total;
    }

    Ok((manifest_json, total_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BundleGitInfo, BundleSelection};
    use std::io::Read;
    use tempfile::tempdir;

    #[test]
    fn writes_zip_with_manifest_and_respects_exclusions() {
        let dir = tempdir().unwrap();
        let repo_root = dir.path();
        std::fs::create_dir_all(repo_root.join("app/src")).unwrap();
        std::fs::create_dir_all(repo_root.join("dist")).unwrap();
        std::fs::write(repo_root.join("README.md"), "root").unwrap();
        std::fs::write(repo_root.join("app/src/main.ts"), "code").unwrap();
        std::fs::write(repo_root.join("dist/bundle.js"), "ignored").unwrap();

        let output_path = repo_root.join("bundle.zip");
        let plan = BundlePlan {
            output_path: output_path.clone(),
            repo_root: repo_root.to_path_buf(),
            repo_id: "repo".to_string(),
            preset_id: "full".to_string(),
            preset_name: "Full".to_string(),
            selection: BundleSelection {
                include_root: true,
                top_level_dirs: vec!["app".to_string()],
                excluded_subdirs: vec![],
            },
            git: BundleGitInfo {
                head_sha: None,
                short_sha: None,
                branch: None,
            },
            built_at_iso: "2026-01-31T00:00:00Z".to_string(),
        };

        let result = write_bundle(&plan).unwrap();
        assert!(result.bytes_written > 0);

        let file = File::open(&output_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        assert!(archive.by_name("README.md").is_ok());
        assert!(archive.by_name("app/src/main.ts").is_ok());
        assert!(archive.by_name("dist/bundle.js").is_err());

        let mut manifest = archive.by_name(MANIFEST_NAME).unwrap();
        let mut manifest_content = String::new();
        manifest.read_to_string(&mut manifest_content).unwrap();
        assert!(manifest_content.contains("\"repoId\""));
        assert!(manifest_content.contains("\"presetId\""));
        assert!(manifest_content.contains("\"fileCount\""));
        assert!(manifest_content.contains("\"totalBytesBestEffort\""));
    }
}
