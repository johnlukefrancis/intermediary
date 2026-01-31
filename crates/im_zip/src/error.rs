// Path: crates/im_zip/src/error.rs
// Description: Error types for zip operations with actionable messages

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during zip operations.
#[derive(Debug, Error)]
pub enum ZipError {
    /// Failed to read the plan file from disk.
    #[error("failed to read plan file at {path}: {source}")]
    PlanReadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse the plan file as JSON.
    #[error("failed to parse plan file: {0}")]
    PlanParseFailed(#[from] serde_json::Error),

    /// Archive path contains path traversal (.. or leading /).
    #[error("path traversal rejected in archive path: {archive_path}")]
    PathTraversal { archive_path: String },

    /// Failed to create the output zip file.
    #[error("failed to create output file at {path}: {source}")]
    OutputCreateFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to read a source file.
    #[error("failed to read source file at {path}: {source}")]
    SourceReadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed during archive write operation.
    #[error("archive write failed: {0}")]
    ArchiveWriteFailed(#[from] zip::result::ZipError),

    /// Failed to finalize the archive.
    #[error("failed to finalize archive: {0}")]
    FinalizeFailed(String),
}

pub type Result<T> = std::result::Result<T, ZipError>;
