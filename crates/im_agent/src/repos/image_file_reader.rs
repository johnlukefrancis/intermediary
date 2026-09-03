// Path: crates/im_agent/src/repos/image_file_reader.rs
// Description: Repo-relative image file reader for in-app preview workspaces

use std::io;
use std::path::Path;
use std::time::UNIX_EPOCH;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use tokio::fs;

use crate::error::AgentError;
use crate::staging::validate_relative_path;

const MAX_IMAGE_FILE_BYTES: u64 = 25 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct ImageFileReadResult {
    pub data_base64: String,
    pub mime_type: String,
    pub bytes: u64,
    pub mtime_ms: u64,
}

pub async fn read_image_file(
    repo_root: &str,
    relative_path: &str,
) -> Result<ImageFileReadResult, AgentError> {
    validate_relative_path(relative_path)?;

    let mime_type = mime_type_for_path(relative_path).ok_or_else(|| {
        AgentError::new(
            "UNSUPPORTED_IMAGE_FILE",
            "Image preview supports PNG, JPEG, WebP, GIF, BMP, and AVIF",
        )
    })?;

    let source_path = Path::new(repo_root).join(relative_path);
    let canonical_root = fs::canonicalize(repo_root)
        .await
        .map_err(|err| AgentError::internal(format!("Failed to resolve repo root: {err}")))?;
    let canonical_source =
        fs::canonicalize(&source_path)
            .await
            .map_err(|err| match err.kind() {
                io::ErrorKind::NotFound => AgentError::new("FILE_NOT_FOUND", "File does not exist"),
                _ => AgentError::internal(format!("Failed to resolve image file: {err}")),
            })?;

    if !canonical_source.starts_with(&canonical_root) {
        return Err(AgentError::new(
            "INVALID_PATH",
            "Path escapes configured repo root",
        ));
    }

    let metadata = fs::metadata(&canonical_source)
        .await
        .map_err(|err| match err.kind() {
            io::ErrorKind::NotFound => AgentError::new("FILE_NOT_FOUND", "File does not exist"),
            _ => AgentError::internal(format!("Failed to stat image file: {err}")),
        })?;

    if !metadata.is_file() {
        return Err(AgentError::new(
            "UNSUPPORTED_IMAGE_FILE",
            "Only regular image files can be opened in the preview",
        ));
    }

    if metadata.len() > MAX_IMAGE_FILE_BYTES {
        return Err(AgentError::new(
            "UNSUPPORTED_IMAGE_FILE",
            "Image file is too large for the preview",
        ));
    }

    let bytes = fs::read(&canonical_source)
        .await
        .map_err(|err| match err.kind() {
            io::ErrorKind::NotFound => AgentError::new("FILE_NOT_FOUND", "File does not exist"),
            _ => AgentError::internal(format!("Failed to read image file: {err}")),
        })?;
    if bytes.len() as u64 > MAX_IMAGE_FILE_BYTES {
        return Err(AgentError::new(
            "UNSUPPORTED_IMAGE_FILE",
            "Image file is too large for the preview",
        ));
    }

    let mtime_ms = metadata
        .modified()
        .ok()
        .and_then(|mtime| mtime.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0);

    Ok(ImageFileReadResult {
        data_base64: STANDARD.encode(bytes),
        mime_type: mime_type.to_string(),
        bytes: metadata.len(),
        mtime_ms,
    })
}

/// The one extension-to-MIME mapping for previewable images; shared with the
/// source-control image diff so both routes accept exactly the same set.
pub(crate) fn mime_type_for_path(relative_path: &str) -> Option<&'static str> {
    let extension = Path::new(relative_path)
        .extension()
        .and_then(|value| value.to_str())?
        .to_ascii_lowercase();

    match extension.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        "bmp" => Some("image/bmp"),
        "avif" => Some("image/avif"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[cfg(unix)]
    use std::os::unix::fs as unix_fs;

    fn repo_root() -> TempDir {
        let root = TempDir::new().expect("tempdir");
        fs::create_dir_all(root.path().join("docs/images")).expect("image dir");
        root
    }

    #[tokio::test]
    async fn reads_supported_image_file() {
        let root = repo_root();
        fs::write(root.path().join("docs/images/capture.png"), b"png bytes").expect("write png");

        let result = read_image_file(
            root.path().to_str().expect("root path"),
            "docs/images/capture.png",
        )
        .await
        .expect("read image");

        assert_eq!(result.mime_type, "image/png");
        assert_eq!(result.data_base64, "cG5nIGJ5dGVz");
        assert_eq!(result.bytes, 9);
        assert!(result.mtime_ms > 0);
    }

    #[tokio::test]
    async fn rejects_traversal_paths() {
        let root = repo_root();
        let err = read_image_file(root.path().to_str().expect("root path"), "../outside.png")
            .await
            .expect_err("traversal should fail");

        assert_eq!(err.code(), "INVALID_PATH");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn rejects_symlink_file_outside_repo_root() {
        let root = repo_root();
        let outside = TempDir::new().expect("outside tempdir");
        fs::write(outside.path().join("secret.png"), b"outside image").expect("write outside");
        unix_fs::symlink(
            outside.path().join("secret.png"),
            root.path().join("docs/images/link.png"),
        )
        .expect("symlink file");

        let err = read_image_file(
            root.path().to_str().expect("root path"),
            "docs/images/link.png",
        )
        .await
        .expect_err("outside symlink should fail");

        assert_eq!(err.code(), "INVALID_PATH");
    }

    #[tokio::test]
    async fn reports_missing_files() {
        let root = repo_root();
        let err = read_image_file(
            root.path().to_str().expect("root path"),
            "docs/images/missing.png",
        )
        .await
        .expect_err("missing file should fail");

        assert_eq!(err.code(), "FILE_NOT_FOUND");
    }

    #[tokio::test]
    async fn rejects_directory_paths() {
        let root = repo_root();
        let err = read_image_file(root.path().to_str().expect("root path"), "docs/images")
            .await
            .expect_err("directory should fail");

        assert_eq!(err.code(), "UNSUPPORTED_IMAGE_FILE");
    }

    #[tokio::test]
    async fn rejects_unsupported_extensions() {
        let root = repo_root();
        fs::write(root.path().join("docs/images/capture.tiff"), b"tiff bytes").expect("write tiff");

        let err = read_image_file(
            root.path().to_str().expect("root path"),
            "docs/images/capture.tiff",
        )
        .await
        .expect_err("unsupported image should fail");

        assert_eq!(err.code(), "UNSUPPORTED_IMAGE_FILE");
    }

    #[tokio::test]
    async fn rejects_oversized_images() {
        let root = repo_root();
        let bytes = vec![0_u8; (MAX_IMAGE_FILE_BYTES + 1) as usize];
        fs::write(root.path().join("docs/images/large.png"), bytes).expect("write large image");

        let err = read_image_file(
            root.path().to_str().expect("root path"),
            "docs/images/large.png",
        )
        .await
        .expect_err("large image should fail");

        assert_eq!(err.code(), "UNSUPPORTED_IMAGE_FILE");
    }
}
