// Path: crates/im_agent/src/repos/image_file_reader.rs
// Description: Repo-relative image file reader for in-app preview workspaces

use std::fs::Metadata;
use std::io;
use std::path::Path;
use std::time::UNIX_EPOCH;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use tokio::fs;
use tokio::io::AsyncReadExt as _;

use crate::error::AgentError;
use crate::staging::validate_relative_path;

/// The preview ceiling every caller inherits: a decoded image this large is
/// already past what a workspace tile can show.
const MAX_IMAGE_FILE_BYTES: u64 = 25 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct ImageFileReadResult {
    pub data_base64: String,
    pub mime_type: String,
    pub bytes: u64,
    pub mtime_ms: u64,
}

/// The unbounded preview read: the process-wide `MAX_IMAGE_FILE_BYTES` alone.
pub async fn read_image_file(
    repo_root: &str,
    relative_path: &str,
) -> Result<ImageFileReadResult, AgentError> {
    read_image_file_bounded(repo_root, relative_path, None).await
}

/// Reads one repo-relative image as base64. `max_bytes` is the caller's own
/// gate (a stream tile's `IMAGE_CARD_MAX_BYTES`): a file over it is refused
/// from the stat, BEFORE any byte is read, so an oversized image never costs a
/// read on either backend. `MAX_IMAGE_FILE_BYTES` still applies on top. The
/// result is bound to one revision: `bytes` is the length actually read and
/// `mtime_ms` comes from a stat taken AFTER the read, which must match the
/// stat taken before it or the read is refused as `Image changed while it was
/// being read`.
pub async fn read_image_file_bounded(
    repo_root: &str,
    relative_path: &str,
    max_bytes: Option<u64>,
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

    let before = ImageStamp::of(&metadata);
    size_gate(before.len, max_bytes)?;

    // Read no more than one byte past the tightest bound: a file that grows
    // between the stat and the read costs that one byte, never the excess.
    let bound = max_bytes.map_or(MAX_IMAGE_FILE_BYTES, |bound| {
        bound.min(MAX_IMAGE_FILE_BYTES)
    });
    let bytes = read_bounded(&canonical_source, bound)
        .await
        .map_err(|err| match err.kind() {
            io::ErrorKind::NotFound => AgentError::new("FILE_NOT_FOUND", "File does not exist"),
            _ => AgentError::internal(format!("Failed to read image file: {err}")),
        })?;
    let read_len = bytes.len() as u64;
    size_gate(read_len, max_bytes)?;

    // The bytes are bound to ONE revision: the stamp after the read must
    // equal the stamp before it, and the read must have seen that whole size.
    let after = fs::metadata(&canonical_source)
        .await
        .map(|metadata| ImageStamp::of(&metadata))
        .map_err(|err| match err.kind() {
            io::ErrorKind::NotFound => AgentError::new("FILE_NOT_FOUND", "File does not exist"),
            _ => AgentError::internal(format!("Failed to re-stat image file: {err}")),
        })?;
    verify_still(&before, &after, read_len)?;

    Ok(ImageFileReadResult {
        data_base64: STANDARD.encode(bytes),
        mime_type: mime_type.to_string(),
        bytes: read_len,
        mtime_ms: after.mtime_ms,
    })
}

/// Size and mtime at one instant; the pair before and after the read proves
/// the bytes belong to a single revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ImageStamp {
    len: u64,
    mtime_ms: u64,
}

impl ImageStamp {
    fn of(metadata: &Metadata) -> Self {
        let mtime_ms = metadata
            .modified()
            .ok()
            .and_then(|mtime| mtime.duration_since(UNIX_EPOCH).ok())
            .map(|since| u64::try_from(since.as_millis()).unwrap_or(u64::MAX))
            .unwrap_or(0);
        Self {
            len: metadata.len(),
            mtime_ms,
        }
    }
}

/// The two size ceilings, applied to the stat before the read and again to
/// the length actually read, so a file that grew in between is still refused.
fn size_gate(len: u64, max_bytes: Option<u64>) -> Result<(), AgentError> {
    if max_bytes.is_some_and(|bound| len > bound) {
        return Err(AgentError::new(
            "UNSUPPORTED_IMAGE_FILE",
            "Image exceeds the requested size bound",
        ));
    }
    if len > MAX_IMAGE_FILE_BYTES {
        return Err(AgentError::new(
            "UNSUPPORTED_IMAGE_FILE",
            "Image file is too large for the preview",
        ));
    }
    Ok(())
}

/// Refuses bytes that do not belong to one revision: the stamp moved across
/// the read, or the read saw a different length than the file has now. The
/// UI treats this refusal as `IMAGE CHANGED` and refetches under the new card.
fn verify_still(before: &ImageStamp, after: &ImageStamp, read_len: u64) -> Result<(), AgentError> {
    if before != after || read_len != after.len {
        return Err(AgentError::new(
            "UNSUPPORTED_IMAGE_FILE",
            "Image changed while it was being read",
        ));
    }
    Ok(())
}

/// Reads at most `bound + 1` bytes; the one byte of overflow is what proves a
/// grown file is over the bound without transferring the rest of it.
async fn read_bounded(path: &Path, bound: u64) -> io::Result<Vec<u8>> {
    let file = fs::File::open(path).await?;
    let mut bytes = Vec::new();
    file.take(bound.saturating_add(1))
        .read_to_end(&mut bytes)
        .await?;
    Ok(bytes)
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
#[path = "image_file_reader_tests.rs"]
mod tests;
