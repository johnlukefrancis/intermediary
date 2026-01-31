// Path: crates/im_bundle/src/error.rs
// Description: Error types for bundle scanning and zip writing

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BundleError {
    #[error("failed to read plan file at {path}: {source}")]
    PlanReadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse plan file: {0}")]
    PlanParseFailed(#[from] serde_json::Error),

    #[error("invalid plan: {0}")]
    InvalidPlan(String),

    #[error("repo root does not exist: {path}")]
    RepoRootMissing { path: PathBuf },

    #[error("top-level directory not found: {dir}")]
    TopLevelDirMissing { dir: String },

    #[error("top-level entry is not a directory: {dir}")]
    TopLevelDirNotDirectory { dir: String },

    #[error("top-level directory is ignored: {dir}")]
    TopLevelDirIgnored { dir: String },

    #[error("failed to read directory at {path}: {source}")]
    DirReadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to read file metadata at {path}: {source}")]
    MetadataFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to open file at {path}: {source}")]
    FileOpenFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to read file at {path}: {source}")]
    FileReadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to create output file at {path}: {source}")]
    OutputCreateFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("archive write failed: {0}")]
    ArchiveWriteFailed(#[from] zip::result::ZipError),

    #[error("failed to finalize archive: {0}")]
    FinalizeFailed(String),
}

pub type Result<T> = std::result::Result<T, BundleError>;
