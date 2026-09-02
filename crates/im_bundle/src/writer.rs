// Path: crates/im_bundle/src/writer.rs
// Description: Bundle zip writer with scanning, manifest, and progress

use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::Instant;

use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::cancel::{check_cancelled, BundleCancelToken};
use crate::error::{BundleError, Result};
use crate::git_capture::{
    GitCaptureSession, WrittenEntryDigests, GIT_DIFF_NAME, GIT_INDEX_DIFF_NAME,
    GIT_OMITTED_PATHS_NAME, GIT_STATUS_NAME, GIT_WORKTREE_DIFF_NAME, HANDOFF_NAME,
};
use crate::manifest::build_manifest;
use crate::plan::BundlePlan;
use crate::progress::ProgressEmitter;
use crate::progress_sink::{ProgressSink, StdoutProgressSink};
use crate::scanner::{scan_bundle_with_cancel, ScanEntry};
use crate::zip_entry::write_entry;

const BUFFER_SIZE: usize = 256 * 1024;
const OUTPUT_BUFFER_SIZE: usize = 256 * 1024;
const MANIFEST_NAME: &str = "BUNDLE_MANIFEST.json";
const GENERATED_ENTRY_COUNT: u64 = 7;
const COMPRESSION_LEVEL: i64 = 6;

#[derive(Debug)]
pub struct BundleResult {
    pub bytes_written: u64,
    pub file_count: u64,
    pub scan_ms: u128,
    pub zip_ms: u128,
    pub git_short_sha: Option<String>,
}

pub fn write_bundle(plan: &BundlePlan) -> Result<BundleResult> {
    write_bundle_with_progress(plan, Box::new(StdoutProgressSink::new()))
}

pub fn write_bundle_with_progress(
    plan: &BundlePlan,
    sink: Box<dyn ProgressSink>,
) -> Result<BundleResult> {
    let mut progress = ProgressEmitter::with_sink(sink);
    write_bundle_with_emitter(plan, &mut progress, None)
}

pub fn write_bundle_with_progress_and_cancel(
    plan: &BundlePlan,
    sink: Box<dyn ProgressSink>,
    cancel_token: &BundleCancelToken,
) -> Result<BundleResult> {
    let mut progress = ProgressEmitter::with_sink(sink);
    write_bundle_with_emitter(plan, &mut progress, Some(cancel_token))
}

fn write_bundle_with_emitter(
    plan: &BundlePlan,
    progress: &mut ProgressEmitter,
    cancel_token: Option<&BundleCancelToken>,
) -> Result<BundleResult> {
    let mut git_session = GitCaptureSession::begin(plan, cancel_token)?;
    let scan_start = Instant::now();
    let scan_result = scan_bundle_with_cancel(plan, progress, cancel_token)?;
    reject_reserved_entry_collisions(&scan_result.entries)?;
    git_session.reconcile_selected_files(&scan_result.entries, cancel_token)?;
    let scan_ms = scan_start.elapsed().as_millis();
    check_cancelled(cancel_token)?;

    let total_files = scan_result.entries.len() as u64 + GENERATED_ENTRY_COUNT;
    progress.emit_progress("zipping", 0, total_files);

    let zip_start = Instant::now();
    let (bytes_written, file_count, git_short_sha) = write_zip(
        plan,
        &scan_result.entries,
        &scan_result.top_level_dirs_included,
        git_session,
        progress,
        cancel_token,
    )?;
    let zip_ms = zip_start.elapsed().as_millis();

    progress.emit_done(bytes_written, file_count, scan_ms, zip_ms);

    Ok(BundleResult {
        bytes_written,
        file_count,
        scan_ms,
        zip_ms,
        git_short_sha,
    })
}

fn write_zip(
    plan: &BundlePlan,
    entries: &[ScanEntry],
    top_level_dirs_included: &[String],
    git_session: GitCaptureSession,
    progress: &mut ProgressEmitter,
    cancel_token: Option<&BundleCancelToken>,
) -> Result<(u64, u64, Option<String>)> {
    check_cancelled(cancel_token)?;
    let output_file =
        File::create(&plan.output_path).map_err(|source| BundleError::OutputCreateFailed {
            path: plan.output_path.clone(),
            source,
        })?;

    let writer = BufWriter::with_capacity(OUTPUT_BUFFER_SIZE, output_file);
    let mut zip = zip::ZipWriter::new(writer);

    let manifest_options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(COMPRESSION_LEVEL));

    let mut bytes_copied = 0u64;
    let mut files_done = 0u64;
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let files_total = entries.len() as u64 + GENERATED_ENTRY_COUNT;
    let watched_paths = git_session.watched_paths();
    let mut written_digests = WrittenEntryDigests::new();

    for entry in entries {
        check_cancelled(cancel_token)?;
        let written = write_entry(
            &mut zip,
            entry,
            &mut buffer,
            progress,
            files_done,
            files_total,
            bytes_copied,
            cancel_token,
            watched_paths.contains(&entry.repo_relative_path),
        )?;
        bytes_copied += written.bytes;
        if let Some(digest) = written.digest {
            written_digests.insert(entry.repo_relative_path.clone(), digest);
        }
        files_done += 1;
        progress.emit_progress("zipping", files_done, files_total);
    }

    check_cancelled(cancel_token)?;
    let git_evidence = git_session.finish(&written_digests, cancel_token)?;
    let generated_bytes = git_evidence.status.len() as u64
        + git_evidence.diff.len() as u64
        + git_evidence.index_diff.len() as u64
        + git_evidence.worktree_diff.len() as u64
        + git_evidence.omitted_paths.len() as u64
        + git_evidence.handoff.len() as u64;
    let (manifest_json, total_bytes_best_effort) = build_manifest_json(
        plan,
        top_level_dirs_included,
        &git_evidence.manifest,
        bytes_copied + generated_bytes,
        entries.len() as u64 + GENERATED_ENTRY_COUNT,
    )?;

    for (name, contents) in [
        (GIT_STATUS_NAME, git_evidence.status.as_slice()),
        (GIT_DIFF_NAME, git_evidence.diff.as_slice()),
        (GIT_INDEX_DIFF_NAME, git_evidence.index_diff.as_slice()),
        (
            GIT_WORKTREE_DIFF_NAME,
            git_evidence.worktree_diff.as_slice(),
        ),
        (
            GIT_OMITTED_PATHS_NAME,
            git_evidence.omitted_paths.as_slice(),
        ),
        (HANDOFF_NAME, git_evidence.handoff.as_slice()),
    ] {
        write_generated_entry(&mut zip, name, contents, manifest_options)?;
        files_done += 1;
        progress.emit_progress("zipping", files_done, files_total);
    }

    write_generated_entry(
        &mut zip,
        MANIFEST_NAME,
        manifest_json.as_bytes(),
        manifest_options,
    )?;
    files_done += 1;
    progress.emit_progress("zipping", files_done, files_total);

    progress.emit_progress("finalizing", files_done, files_total);
    check_cancelled(cancel_token)?;
    let writer = zip
        .finish()
        .map_err(|e| BundleError::FinalizeFailed(format!("failed to finish archive: {e}")))?;
    let file = writer
        .into_inner()
        .map_err(|e| BundleError::FinalizeFailed(format!("failed to flush buffer: {e}")))?;

    progress.emit_progress("syncing", files_done, files_total);
    check_cancelled(cancel_token)?;
    file.sync_all()
        .map_err(|e| BundleError::FinalizeFailed(format!("failed to sync file: {e}")))?;

    let bytes_written = file.metadata().map(|m| m.len()).unwrap_or(0);

    if total_bytes_best_effort != bytes_copied + manifest_json.len() as u64 {
        // Keep calculation local; manifest already written.
    }

    Ok((bytes_written, files_done, git_evidence.manifest.short_sha))
}

fn build_manifest_json(
    plan: &BundlePlan,
    top_level_dirs_included: &[String],
    git: &crate::git_capture::BundleGitCapture,
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
            &plan.global_excludes,
            top_level_dirs_included,
            git,
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

fn write_generated_entry(
    zip: &mut zip::ZipWriter<BufWriter<File>>,
    name: &str,
    contents: &[u8],
    options: SimpleFileOptions,
) -> Result<()> {
    zip.start_file(name, options)
        .map_err(|source| BundleError::ArchiveWriteFailed {
            archive_path: name.to_string(),
            source,
        })?;
    zip.write_all(contents).map_err(|error| {
        BundleError::FinalizeFailed(format!("failed to write generated entry {name}: {error}"))
    })
}

fn reject_reserved_entry_collisions(entries: &[ScanEntry]) -> Result<()> {
    let reserved = [
        MANIFEST_NAME,
        GIT_STATUS_NAME,
        GIT_DIFF_NAME,
        GIT_INDEX_DIFF_NAME,
        GIT_WORKTREE_DIFF_NAME,
        GIT_OMITTED_PATHS_NAME,
        HANDOFF_NAME,
    ];
    if let Some(entry) = entries
        .iter()
        .find(|entry| reserved.contains(&entry.archive_path.as_str()))
    {
        return Err(BundleError::ReservedEntryCollision {
            archive_path: entry.archive_path.clone(),
        });
    }
    Ok(())
}
