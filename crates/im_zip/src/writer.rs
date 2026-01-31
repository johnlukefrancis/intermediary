// Path: crates/im_zip/src/writer.rs
// Description: Core zip writing logic with streaming and progress

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

use crate::error::{Result, ZipError};
use crate::plan::ZipPlan;
use crate::progress::ProgressReporter;

/// Buffer size for streaming file reads (64KB).
const BUFFER_SIZE: usize = 64 * 1024;

/// Manifest file name in the archive.
const MANIFEST_NAME: &str = "INTERMEDIARY_MANIFEST.json";

/// Compression level (matches Node archiver default).
const COMPRESSION_LEVEL: i64 = 6;

/// Write a zip archive according to the given plan.
pub fn write_zip(plan: &ZipPlan) -> Result<u64> {
    let output_file = File::create(&plan.output_path).map_err(|source| {
        ZipError::OutputCreateFailed {
            path: plan.output_path.clone(),
            source,
        }
    })?;

    let writer = BufWriter::new(output_file);
    let mut zip = zip::ZipWriter::new(writer);

    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(COMPRESSION_LEVEL));

    // Calculate total files for progress (entries + optional manifest)
    let total_files = plan.entries.len() as u64 + plan.manifest_json.as_ref().map_or(0, |_| 1);
    let mut progress = ProgressReporter::new(total_files);

    // Write manifest first if present
    if let Some(ref manifest) = plan.manifest_json {
        zip.start_file(MANIFEST_NAME, options)?;
        zip.write_all(manifest.as_bytes()).map_err(|e| {
            ZipError::FinalizeFailed(format!("failed to write manifest: {}", e))
        })?;
        progress.file_done();
    }

    // Write each entry
    let mut buffer = vec![0u8; BUFFER_SIZE];
    for entry in &plan.entries {
        write_entry(&mut zip, &entry.source_path, &entry.archive_path, options, &mut buffer)?;
        progress.file_done();
    }

    // Finalize and sync
    let writer = zip.finish()?;
    let file = writer.into_inner().map_err(|e| {
        ZipError::FinalizeFailed(format!("failed to flush buffer: {}", e))
    })?;

    file.sync_all().map_err(|e| {
        ZipError::FinalizeFailed(format!("failed to sync file: {}", e))
    })?;

    // Get final size
    let bytes_written = file.metadata().map(|m| m.len()).unwrap_or(0);
    progress.emit_done(bytes_written);

    Ok(bytes_written)
}

/// Write a single file entry to the archive.
fn write_entry(
    zip: &mut zip::ZipWriter<BufWriter<File>>,
    source_path: &Path,
    archive_path: &str,
    options: SimpleFileOptions,
    buffer: &mut [u8],
) -> Result<()> {
    // Normalize backslashes to forward slashes for cross-platform compatibility
    let normalized_path = archive_path.replace('\\', "/");

    let source_file = File::open(source_path).map_err(|source| ZipError::SourceReadFailed {
        path: source_path.to_path_buf(),
        source,
    })?;

    let mut reader = BufReader::new(source_file);

    zip.start_file(&normalized_path, options)?;

    // Stream the file in chunks
    loop {
        let bytes_read = reader.read(buffer).map_err(|source| ZipError::SourceReadFailed {
            path: source_path.to_path_buf(),
            source,
        })?;

        if bytes_read == 0 {
            break;
        }

        zip.write_all(&buffer[..bytes_read]).map_err(|e| {
            ZipError::FinalizeFailed(format!("failed to write to archive: {}", e))
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::PlanEntry;
    use std::io::Read;
    use tempfile::tempdir;

    #[test]
    fn write_archive_with_files_and_manifest() {
        let dir = tempdir().unwrap();

        // Create source files
        let source1 = dir.path().join("file1.txt");
        std::fs::write(&source1, "content of file 1").unwrap();

        let source2 = dir.path().join("file2.txt");
        std::fs::write(&source2, "content of file 2").unwrap();

        // Create plan
        let output_path = dir.path().join("output.zip");
        let plan = ZipPlan {
            output_path: output_path.clone(),
            entries: vec![
                PlanEntry {
                    source_path: source1,
                    archive_path: "dir/file1.txt".to_string(),
                    size_bytes: None,
                },
                PlanEntry {
                    source_path: source2,
                    archive_path: "file2.txt".to_string(),
                    size_bytes: Some(18),
                },
            ],
            manifest_json: Some(r#"{"generatedAt":"2026-01-31"}"#.to_string()),
        };

        // Write the archive
        let bytes = write_zip(&plan).unwrap();
        assert!(bytes > 0);
        assert!(output_path.exists());

        // Verify contents
        let file = File::open(&output_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();

        // Check manifest (scope to release borrow)
        {
            let mut manifest = archive.by_name(MANIFEST_NAME).unwrap();
            let mut content = String::new();
            manifest.read_to_string(&mut content).unwrap();
            assert!(content.contains("generatedAt"));
        }

        // Check file1
        {
            let mut f1 = archive.by_name("dir/file1.txt").unwrap();
            let mut content = String::new();
            f1.read_to_string(&mut content).unwrap();
            assert_eq!(content, "content of file 1");
        }

        // Check file2
        {
            let mut f2 = archive.by_name("file2.txt").unwrap();
            let mut content = String::new();
            f2.read_to_string(&mut content).unwrap();
            assert_eq!(content, "content of file 2");
        }
    }

    #[test]
    fn write_empty_archive_with_manifest_only() {
        let dir = tempdir().unwrap();
        let output_path = dir.path().join("empty.zip");

        let plan = ZipPlan {
            output_path: output_path.clone(),
            entries: vec![],
            manifest_json: Some(r#"{"test":true}"#.to_string()),
        };

        let bytes = write_zip(&plan).unwrap();
        assert!(bytes > 0);

        let file = File::open(&output_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        assert_eq!(archive.len(), 1);
        assert!(archive.by_name(MANIFEST_NAME).is_ok());
    }

    #[test]
    fn normalizes_backslashes() {
        let dir = tempdir().unwrap();

        let source = dir.path().join("file.txt");
        std::fs::write(&source, "test").unwrap();

        let output_path = dir.path().join("output.zip");
        let plan = ZipPlan {
            output_path: output_path.clone(),
            entries: vec![PlanEntry {
                source_path: source,
                archive_path: "dir\\subdir\\file.txt".to_string(),
                size_bytes: None,
            }],
            manifest_json: None,
        };

        write_zip(&plan).unwrap();

        let file = File::open(&output_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();

        // Should be accessible with forward slashes
        assert!(archive.by_name("dir/subdir/file.txt").is_ok());
    }
}
