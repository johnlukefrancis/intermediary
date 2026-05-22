// Path: crates/im_bundle/src/zip_entry.rs
// Description: Single file entry writer for bundle zip archives

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};

use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::cancel::{check_cancelled, BundleCancelToken};
use crate::compression_policy::compression_method_for;
use crate::error::{BundleError, Result};
use crate::progress::ProgressEmitter;
use crate::scanner::ScanEntry;

const COMPRESSION_LEVEL: i64 = 6;

pub(crate) fn write_entry(
    zip: &mut zip::ZipWriter<BufWriter<File>>,
    entry: &ScanEntry,
    buffer: &mut [u8],
    progress: &mut ProgressEmitter,
    files_done: u64,
    files_total: u64,
    bytes_done_total: u64,
    cancel_token: Option<&BundleCancelToken>,
) -> Result<u64> {
    check_cancelled(cancel_token)?;
    let source_file =
        File::open(&entry.source_path).map_err(|source| BundleError::FileOpenFailed {
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

    zip.start_file(
        &entry.archive_path,
        build_entry_options(&entry.archive_path, current_bytes_total),
    )
    .map_err(|source| BundleError::ArchiveWriteFailed {
        archive_path: entry.archive_path.clone(),
        source,
    })?;

    let mut total = 0u64;
    emit_entry_progress(
        progress,
        entry,
        files_done,
        files_total,
        0,
        current_bytes_total,
        bytes_done_total,
        true,
    );
    loop {
        check_cancelled(cancel_token)?;
        let bytes_read = reader
            .read(buffer)
            .map_err(|source| BundleError::FileReadFailed {
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
        emit_entry_progress(
            progress,
            entry,
            files_done,
            files_total,
            total,
            current_bytes_total,
            bytes_done_total + total,
            false,
        );
    }

    emit_entry_progress(
        progress,
        entry,
        files_done,
        files_total,
        total,
        current_bytes_total,
        bytes_done_total + total,
        true,
    );

    Ok(total)
}

fn emit_entry_progress(
    progress: &mut ProgressEmitter,
    entry: &ScanEntry,
    files_done: u64,
    files_total: u64,
    current_bytes_done: u64,
    current_bytes_total: u64,
    bytes_done_total: u64,
    completed: bool,
) {
    progress.emit_file_progress(
        "zipping",
        files_done,
        files_total,
        &entry.archive_path,
        current_bytes_done,
        Some(current_bytes_total),
        bytes_done_total,
        completed,
    );
}

fn build_entry_options(archive_path: &str, size_bytes: u64) -> SimpleFileOptions {
    let method = compression_method_for(archive_path, size_bytes);
    let mut options = SimpleFileOptions::default().compression_method(method);
    if method == CompressionMethod::Deflated {
        options = options.compression_level(Some(COMPRESSION_LEVEL));
    }
    options
}
