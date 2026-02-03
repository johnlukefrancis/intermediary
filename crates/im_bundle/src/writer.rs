// Path: crates/im_bundle/src/writer.rs
// Description: Bundle zip writer with scanning, manifest, and progress

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::time::Instant;

use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::compression_policy::compression_method_for;
use crate::error::{BundleError, Result};
use crate::manifest::build_manifest;
use crate::plan::BundlePlan;
use crate::progress::ProgressEmitter;
use crate::progress_sink::{ProgressSink, StdoutProgressSink};
use crate::scanner::{scan_bundle, ScanEntry};

const BUFFER_SIZE: usize = 256 * 1024;
const OUTPUT_BUFFER_SIZE: usize = 256 * 1024;
const MANIFEST_NAME: &str = "BUNDLE_MANIFEST.json";
const COMPRESSION_LEVEL: i64 = 6;

#[derive(Debug)]
pub struct BundleResult {
    pub bytes_written: u64,
    pub file_count: u64,
    pub scan_ms: u128,
    pub zip_ms: u128,
}

pub fn write_bundle(plan: &BundlePlan) -> Result<BundleResult> {
    write_bundle_with_progress(plan, Box::new(StdoutProgressSink::new()))
}

pub fn write_bundle_with_progress(
    plan: &BundlePlan,
    sink: Box<dyn ProgressSink>,
) -> Result<BundleResult> {
    let mut progress = ProgressEmitter::with_sink(sink);
    write_bundle_with_emitter(plan, &mut progress)
}

fn write_bundle_with_emitter(plan: &BundlePlan, progress: &mut ProgressEmitter) -> Result<BundleResult> {

    let scan_start = Instant::now();
    let scan_result = scan_bundle(plan, progress)?;
    let scan_ms = scan_start.elapsed().as_millis();

    let total_files = scan_result.entries.len() as u64 + 1;
    progress.emit_progress("zipping", 0, total_files);

    let zip_start = Instant::now();
    let (bytes_written, file_count) = write_zip(
        plan,
        &scan_result.entries,
        &scan_result.top_level_dirs_included,
        progress,
    )?;
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

    let writer = BufWriter::with_capacity(OUTPUT_BUFFER_SIZE, output_file);
    let mut zip = zip::ZipWriter::new(writer);

    let manifest_options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(COMPRESSION_LEVEL));

    let mut bytes_copied = 0u64;
    let mut files_done = 0u64;
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let files_total = entries.len() as u64 + 1;

    for entry in entries {
        bytes_copied += write_entry(
            &mut zip,
            entry,
            &mut buffer,
            progress,
            files_done,
            files_total,
            bytes_copied,
        )?;
        files_done += 1;
        progress.emit_progress("zipping", files_done, files_total);
    }

    let (manifest_json, total_bytes_best_effort) = build_manifest_json(
        plan,
        top_level_dirs_included,
        bytes_copied,
        entries.len() as u64 + 1,
    )?;

    zip.start_file(MANIFEST_NAME, manifest_options)
        .map_err(|source| BundleError::ArchiveWriteFailed {
        archive_path: MANIFEST_NAME.to_string(),
        source,
    })?;
    zip.write_all(manifest_json.as_bytes())
        .map_err(|e| BundleError::FinalizeFailed(format!("failed to write manifest entry {MANIFEST_NAME}: {e}")))?;
    files_done += 1;
    progress.emit_progress("zipping", files_done, files_total);

    let writer = zip
        .finish()
        .map_err(|e| BundleError::FinalizeFailed(format!("failed to finish archive: {e}")))?;
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
    buffer: &mut [u8],
    progress: &mut ProgressEmitter,
    files_done: u64,
    files_total: u64,
    bytes_done_total: u64,
) -> Result<u64> {
    let source_file = File::open(&entry.source_path).map_err(|source| BundleError::FileOpenFailed {
        path: entry.source_path.clone(),
        archive_path: entry.archive_path.clone(),
        source,
    })?;
    let current_bytes_total = source_file
        .metadata()
        .map(|metadata| metadata.len())
        .map_err(|source| BundleError::FileMetadataFailed {
            path: entry.source_path.clone(),
            archive_path: entry.archive_path.clone(),
            source,
        })?;
    let mut reader = BufReader::new(source_file).take(current_bytes_total);

    let entry_options = build_entry_options(&entry.archive_path, current_bytes_total);

    zip.start_file(&entry.archive_path, entry_options)
        .map_err(|source| BundleError::ArchiveWriteFailed {
        archive_path: entry.archive_path.clone(),
        source,
    })?;

    let mut total = 0u64;
    progress.emit_file_progress(
        "zipping",
        files_done,
        files_total,
        &entry.archive_path,
        total,
        Some(current_bytes_total),
        bytes_done_total,
        true,
    );
    loop {
        let bytes_read = reader.read(buffer).map_err(|source| BundleError::FileReadFailed {
            path: entry.source_path.clone(),
            archive_path: entry.archive_path.clone(),
            source,
        })?;
        if bytes_read == 0 {
            break;
        }
        zip.write_all(&buffer[..bytes_read])
            .map_err(|source| BundleError::ArchiveWriteFailed {
                archive_path: entry.archive_path.clone(),
                source: zip::result::ZipError::Io(source),
            })?;
        total += bytes_read as u64;
        progress.emit_file_progress(
            "zipping",
            files_done,
            files_total,
            &entry.archive_path,
            total,
            Some(current_bytes_total),
            bytes_done_total + total,
            false,
        );
    }

    progress.emit_file_progress(
        "zipping",
        files_done,
        files_total,
        &entry.archive_path,
        total,
        Some(current_bytes_total),
        bytes_done_total + total,
        true,
    );

    Ok(total)
}

fn build_entry_options(archive_path: &str, size_bytes: u64) -> SimpleFileOptions {
    let method = compression_method_for(archive_path, size_bytes);
    let mut options = SimpleFileOptions::default().compression_method(method);
    if method == CompressionMethod::Deflated {
        options = options.compression_level(Some(COMPRESSION_LEVEL));
    }
    options
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
