// Path: crates/im_agent/src/repos/text_file_reader.rs
// Description: Repo-relative UTF-8 text file reader for in-app scratch viewing

use std::io;
use std::path::Path;
use std::time::UNIX_EPOCH;

use tokio::fs;

use crate::error::AgentError;
use crate::staging::validate_relative_path;

const MAX_TEXT_FILE_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone)]
pub struct TextFileReadResult {
    pub content: String,
    pub bytes: u64,
    pub mtime_ms: u64,
}

pub async fn read_text_file(
    repo_root: &str,
    relative_path: &str,
) -> Result<TextFileReadResult, AgentError> {
    validate_relative_path(relative_path)?;

    let source_path = Path::new(repo_root).join(relative_path);
    let canonical_root = fs::canonicalize(repo_root)
        .await
        .map_err(|err| AgentError::internal(format!("Failed to resolve repo root: {err}")))?;
    let canonical_source =
        fs::canonicalize(&source_path)
            .await
            .map_err(|err| match err.kind() {
                io::ErrorKind::NotFound => AgentError::new("FILE_NOT_FOUND", "File does not exist"),
                _ => AgentError::internal(format!("Failed to resolve text file: {err}")),
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
            _ => AgentError::internal(format!("Failed to stat text file: {err}")),
        })?;

    if !metadata.is_file() {
        return Err(AgentError::new(
            "UNSUPPORTED_TEXT_FILE",
            "Only regular text files can be opened in the viewer",
        ));
    }

    if metadata.len() > MAX_TEXT_FILE_BYTES {
        return Err(AgentError::new(
            "UNSUPPORTED_TEXT_FILE",
            "File is too large for the text viewer",
        ));
    }

    let bytes = fs::read(&canonical_source)
        .await
        .map_err(|err| match err.kind() {
            io::ErrorKind::NotFound => AgentError::new("FILE_NOT_FOUND", "File does not exist"),
            _ => AgentError::internal(format!("Failed to read text file: {err}")),
        })?;
    if bytes.len() as u64 > MAX_TEXT_FILE_BYTES {
        return Err(AgentError::new(
            "UNSUPPORTED_TEXT_FILE",
            "File is too large for the text viewer",
        ));
    }
    if bytes.contains(&0) {
        return Err(AgentError::new(
            "UNSUPPORTED_TEXT_FILE",
            "File is not valid text",
        ));
    }

    let content = String::from_utf8(bytes)
        .map_err(|_| AgentError::new("UNSUPPORTED_TEXT_FILE", "File is not valid UTF-8 text"))?;
    let mtime_ms = metadata
        .modified()
        .ok()
        .and_then(|mtime| mtime.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0);

    Ok(TextFileReadResult {
        bytes: metadata.len(),
        content,
        mtime_ms,
    })
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
        fs::create_dir_all(root.path().join("src")).expect("src dir");
        root
    }

    #[tokio::test]
    async fn reads_utf8_text_file() {
        let root = repo_root();
        fs::write(root.path().join("src/main.ts"), "const answer = 42;\n").expect("write text");

        let result = read_text_file(root.path().to_str().expect("root path"), "src/main.ts")
            .await
            .expect("read text");

        assert_eq!(result.content, "const answer = 42;\n");
        assert_eq!(result.bytes, 19);
        assert!(result.mtime_ms > 0);
    }

    #[tokio::test]
    async fn rejects_traversal_paths() {
        let root = repo_root();
        let err = read_text_file(root.path().to_str().expect("root path"), "../outside.txt")
            .await
            .expect_err("traversal should fail");

        assert_eq!(err.code(), "INVALID_PATH");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn rejects_symlink_file_outside_repo_root() {
        let root = repo_root();
        let outside = TempDir::new().expect("outside tempdir");
        fs::write(outside.path().join("secret.txt"), "outside text").expect("write outside file");
        unix_fs::symlink(
            outside.path().join("secret.txt"),
            root.path().join("src/link.txt"),
        )
        .expect("symlink file");

        let err = read_text_file(root.path().to_str().expect("root path"), "src/link.txt")
            .await
            .expect_err("outside symlink should fail");

        assert_eq!(err.code(), "INVALID_PATH");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn rejects_symlink_directory_outside_repo_root() {
        let root = repo_root();
        let outside = TempDir::new().expect("outside tempdir");
        fs::write(outside.path().join("secret.txt"), "outside text").expect("write outside file");
        unix_fs::symlink(outside.path(), root.path().join("src/outside"))
            .expect("symlink directory");

        let err = read_text_file(
            root.path().to_str().expect("root path"),
            "src/outside/secret.txt",
        )
        .await
        .expect_err("outside symlink directory should fail");

        assert_eq!(err.code(), "INVALID_PATH");
    }

    #[tokio::test]
    async fn rejects_binary_nul_files() {
        let root = repo_root();
        fs::write(root.path().join("src/blob.bin"), b"abc\0def").expect("write binary");

        let err = read_text_file(root.path().to_str().expect("root path"), "src/blob.bin")
            .await
            .expect_err("binary should fail");

        assert_eq!(err.code(), "UNSUPPORTED_TEXT_FILE");
    }

    #[tokio::test]
    async fn reports_missing_files() {
        let root = repo_root();
        let err = read_text_file(root.path().to_str().expect("root path"), "src/missing.txt")
            .await
            .expect_err("missing file should fail");

        assert_eq!(err.code(), "FILE_NOT_FOUND");
    }

    #[tokio::test]
    async fn rejects_invalid_utf8_files() {
        let root = repo_root();
        fs::write(root.path().join("src/blob.txt"), [0xff, 0xfe]).expect("write invalid utf8");

        let err = read_text_file(root.path().to_str().expect("root path"), "src/blob.txt")
            .await
            .expect_err("invalid utf8 should fail");

        assert_eq!(err.code(), "UNSUPPORTED_TEXT_FILE");
    }

    #[tokio::test]
    async fn rejects_oversized_files() {
        let root = repo_root();
        let bytes = vec![b'a'; (MAX_TEXT_FILE_BYTES + 1) as usize];
        fs::write(root.path().join("src/large.txt"), bytes).expect("write large file");

        let err = read_text_file(root.path().to_str().expect("root path"), "src/large.txt")
            .await
            .expect_err("large file should fail");

        assert_eq!(err.code(), "UNSUPPORTED_TEXT_FILE");
    }
}
