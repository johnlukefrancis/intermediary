// Path: crates/im_agent/src/source_control/diff/image_sides.rs
// Description: Reads one image-diff side from a Git blob or from the working tree, bounded and base64-encoded

use std::io;
use std::path::Path;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use im_bundle::git::BundleCancelToken;
use tokio::fs;

use crate::error::AgentError;
use crate::protocol::{ImageDiffSide, ImageDiffSource};
use crate::repos::read_image_file;

use crate::source_control::runner::{self, GitCall, IMAGE_DIFF_SIDE_LIMIT, READ_TIMEOUT};

/// Reads one Git blob (`HEAD:path`, `:0:path`, `:2:path`, ...) as raw bytes and
/// encodes it for the wire. An empty blob is `None`: `git add -N` stages the
/// empty blob for an intent-to-add path, and no empty blob is a picture. A blob
/// past the bound comes back as a `truncated` side carrying the bound instead
/// of bytes, so the UI can say so rather than render a half image.
pub(super) async fn blob_side(
    repo_root: &Path,
    spec: &str,
    source: ImageDiffSource,
    mime_type: &str,
    cancel_token: Option<BundleCancelToken>,
) -> Result<Option<ImageDiffSide>, AgentError> {
    let call = GitCall::new(["show"])
        .arg(spec)
        .stdout_limit(IMAGE_DIFF_SIDE_LIMIT)
        .timeout(READ_TIMEOUT);
    let output = runner::run_read(repo_root, call, cancel_token).await?;
    if output.stdout_truncated {
        return Ok(Some(truncated_side(source, mime_type)));
    }
    if output.stdout.is_empty() {
        return Ok(None);
    }
    Ok(Some(ImageDiffSide {
        source,
        data_base64: STANDARD.encode(&output.stdout),
        mime_type: mime_type.to_string(),
        bytes: output.stdout.len() as u64,
        truncated: false,
    }))
}

/// Whether a `<rev>:<path>` spec resolves. `cat-file -e` answers 1 for an
/// object Git does not have and 128 for a path missing from the tree or a HEAD
/// that does not exist yet (an unborn branch); all three mean "no such side".
pub(super) async fn head_blob_exists(
    repo_root: &Path,
    spec: &str,
    cancel_token: Option<BundleCancelToken>,
) -> Result<bool, AgentError> {
    let call = GitCall::new(["cat-file", "-e"])
        .arg(spec)
        .accept_exit_codes(&[1, 128])
        .timeout(READ_TIMEOUT);
    let output = runner::run_read(repo_root, call, cancel_token).await?;
    Ok(output.exit_code == 0)
}

/// Reads the working-tree file through the one image reader the preview uses,
/// so path containment, regular-file, and MIME rules stay in a single owner. A
/// file that is not on disk is a missing side, not an error. A regular file
/// past the per-side bound is reported as truncated before it is read.
pub(super) async fn worktree_side(
    repo_root: &Path,
    path: &str,
    mime_type: &str,
) -> Result<Option<ImageDiffSide>, AgentError> {
    let repo_root_str = repo_root.to_str().ok_or_else(|| {
        AgentError::new(
            "INVALID_REPO",
            format!("Repo root is not valid UTF-8: {}", repo_root.display()),
        )
    })?;
    // Stat through symlinks, as the reader does; the post-read check below is the real bound.
    match fs::metadata(repo_root.join(path)).await {
        Ok(metadata) if metadata.is_file() && metadata.len() > IMAGE_DIFF_SIDE_LIMIT as u64 => {
            return Ok(Some(truncated_side(ImageDiffSource::Worktree, mime_type)));
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(AgentError::internal(format!(
                "Failed to inspect {path}: {error}"
            )))
        }
    }
    match read_image_file(repo_root_str, path).await {
        Ok(result) if result.bytes > IMAGE_DIFF_SIDE_LIMIT as u64 => {
            Ok(Some(truncated_side(ImageDiffSource::Worktree, mime_type)))
        }
        Ok(result) => Ok(Some(ImageDiffSide {
            source: ImageDiffSource::Worktree,
            data_base64: result.data_base64,
            mime_type: result.mime_type,
            bytes: result.bytes,
            truncated: false,
        })),
        Err(error) if error.code() == "FILE_NOT_FOUND" => Ok(None),
        Err(error) => Err(error),
    }
}

fn truncated_side(source: ImageDiffSource, mime_type: &str) -> ImageDiffSide {
    ImageDiffSide {
        source,
        data_base64: String::new(),
        mime_type: mime_type.to_string(),
        bytes: IMAGE_DIFF_SIDE_LIMIT as u64,
        truncated: true,
    }
}
