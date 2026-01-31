// Path: crates/im_zip/src/plan.rs
// Description: ZipPlan schema and validation for plan file parsing

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Result, ZipError};

/// A single entry in the zip plan.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanEntry {
    /// Absolute path to the source file on disk.
    pub source_path: PathBuf,
    /// Path within the archive (forward slashes).
    pub archive_path: String,
    /// Optional size hint for progress reporting.
    pub size_bytes: Option<u64>,
}

/// The complete zip plan describing what to archive.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZipPlan {
    /// Where to write the output zip file.
    pub output_path: PathBuf,
    /// Files to include in the archive.
    pub entries: Vec<PlanEntry>,
    /// Optional manifest JSON to include as INTERMEDIARY_MANIFEST.json.
    pub manifest_json: Option<String>,
}

impl ZipPlan {
    /// Load and validate a zip plan from a JSON file.
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|source| {
            ZipError::PlanReadFailed {
                path: path.to_path_buf(),
                source,
            }
        })?;

        let plan: ZipPlan = serde_json::from_str(&content)?;
        plan.validate()?;
        Ok(plan)
    }

    /// Validate all archive paths for security issues.
    fn validate(&self) -> Result<()> {
        for entry in &self.entries {
            validate_archive_path(&entry.archive_path)?;
        }
        Ok(())
    }
}

/// Reject archive paths that could escape the archive root.
fn validate_archive_path(archive_path: &str) -> Result<()> {
    // Reject leading slash (absolute path)
    if archive_path.starts_with('/') || archive_path.starts_with('\\') {
        return Err(ZipError::PathTraversal {
            archive_path: archive_path.to_string(),
        });
    }

    // Reject path traversal components
    for component in archive_path.split(['/', '\\']) {
        if component == ".." {
            return Err(ZipError::PathTraversal {
                archive_path: archive_path.to_string(),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn parse_valid_plan() {
        let json = r#"{
            "outputPath": "/tmp/test.zip",
            "entries": [
                {"sourcePath": "/src/file.txt", "archivePath": "dir/file.txt", "sizeBytes": 100}
            ],
            "manifestJson": "{\"test\": true}"
        }"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(json.as_bytes()).unwrap();

        let plan = ZipPlan::load(file.path()).unwrap();
        assert_eq!(plan.output_path, PathBuf::from("/tmp/test.zip"));
        assert_eq!(plan.entries.len(), 1);
        assert_eq!(plan.entries[0].archive_path, "dir/file.txt");
        assert_eq!(plan.entries[0].size_bytes, Some(100));
        assert!(plan.manifest_json.is_some());
    }

    #[test]
    fn parse_minimal_plan() {
        let json = r#"{
            "outputPath": "/tmp/test.zip",
            "entries": []
        }"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(json.as_bytes()).unwrap();

        let plan = ZipPlan::load(file.path()).unwrap();
        assert!(plan.entries.is_empty());
        assert!(plan.manifest_json.is_none());
    }

    #[test]
    fn reject_path_traversal_dotdot() {
        let json = r#"{
            "outputPath": "/tmp/test.zip",
            "entries": [
                {"sourcePath": "/src/file.txt", "archivePath": "../escape.txt"}
            ]
        }"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(json.as_bytes()).unwrap();

        let result = ZipPlan::load(file.path());
        assert!(matches!(result, Err(ZipError::PathTraversal { .. })));
    }

    #[test]
    fn reject_path_traversal_leading_slash() {
        let json = r#"{
            "outputPath": "/tmp/test.zip",
            "entries": [
                {"sourcePath": "/src/file.txt", "archivePath": "/absolute/path.txt"}
            ]
        }"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(json.as_bytes()).unwrap();

        let result = ZipPlan::load(file.path());
        assert!(matches!(result, Err(ZipError::PathTraversal { .. })));
    }

    #[test]
    fn reject_embedded_dotdot() {
        let json = r#"{
            "outputPath": "/tmp/test.zip",
            "entries": [
                {"sourcePath": "/src/file.txt", "archivePath": "dir/../../../escape.txt"}
            ]
        }"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(json.as_bytes()).unwrap();

        let result = ZipPlan::load(file.path());
        assert!(matches!(result, Err(ZipError::PathTraversal { .. })));
    }
}
