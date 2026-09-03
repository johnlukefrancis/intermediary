// Path: crates/im_agent/src/source_control/diff/image.rs
// Description: Chooses the before/after Git snapshots of one changed image and assembles both sides

use std::path::Path;

use im_bundle::git::BundleCancelToken;

use crate::error::AgentError;
use crate::protocol::{ImageDiffSource, SourceControlArea};
use crate::repos::mime_type_for_path;

use super::image_sides::{blob_side, head_blob_exists, worktree_side};

/// `<rev>:<path>` and `:<n>:<path>` resolve against the working tree's top level, but UI
/// paths are relative to the configured repo root, which may sit below it. Git's `./` form
/// resolves against the process cwd, which is that root; it is a no-op for a top-level root.
fn blob_spec(prefix: &str, path: &str) -> String {
    format!("{prefix}./{path}")
}
use crate::source_control::paths::normalize_path;
use crate::source_control::runner::{self, GitCall};
use crate::source_control::SourceControlImageDiff;

/// Index stages Git holds for one path. Stage 0 is the ordinary staged entry;
/// stages 1/2/3 are base/ours/theirs of an unresolved merge.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct IndexStages {
    staged: bool,
    ours: bool,
    theirs: bool,
}

impl IndexStages {
    fn conflicted(self) -> bool {
        self.ours || self.theirs
    }
}

/// Reads both snapshots of one changed image. A snapshot that does not exist
/// (added, deleted, an unborn HEAD, a stage Git never wrote) is `None`, never
/// an error: only Git itself failing, an invalid path, or an extension the
/// preview cannot render is reported as an error.
///
/// A conflicted path is decided by the index, not by the requested area: the
/// UI sends conflicts as `Worktree`, and stages 2/3 outrank that.
pub(in crate::source_control) async fn capture_image_diff(
    repo_root: &Path,
    path: &str,
    original_path: Option<&str>,
    area: SourceControlArea,
    cancel_token: Option<BundleCancelToken>,
) -> Result<SourceControlImageDiff, AgentError> {
    let path = normalize_path(path)?;
    let original = original_path.map(normalize_path).transpose()?;
    let mime_type = mime_type_for_path(&path).ok_or_else(|| {
        AgentError::new(
            "UNSUPPORTED_IMAGE_FILE",
            "Image diff supports PNG, JPEG, WebP, GIF, BMP, and AVIF",
        )
    })?;

    let stages = index_stages(repo_root, &path, cancel_token.clone()).await?;
    if stages.conflicted() {
        return conflict_sides(repo_root, &path, mime_type, stages, cancel_token).await;
    }
    match area {
        SourceControlArea::Index => {
            index_sides(repo_root, &path, original.as_deref(), mime_type, stages, cancel_token).await
        }
        SourceControlArea::Worktree => {
            worktree_sides(repo_root, &path, mime_type, stages, cancel_token).await
        }
    }
}

/// `:2:` is our side of the merge and `:3:` theirs; a delete/modify conflict
/// leaves one of them absent.
async fn conflict_sides(
    repo_root: &Path,
    path: &str,
    mime_type: &str,
    stages: IndexStages,
    cancel_token: Option<BundleCancelToken>,
) -> Result<SourceControlImageDiff, AgentError> {
    let before = if stages.ours {
        blob_side(repo_root, &blob_spec(":2:", path), ImageDiffSource::Ours, mime_type, cancel_token.clone()).await?
    } else {
        None
    };
    let after = if stages.theirs {
        blob_side(repo_root, &blob_spec(":3:", path), ImageDiffSource::Theirs, mime_type, cancel_token).await?
    } else {
        None
    };
    Ok(SourceControlImageDiff { before, after })
}

/// Staged change: committed snapshot against the index. A rename reads the old
/// path out of HEAD; a rename whose source was not a previewable image has no
/// previous picture to show, so that side is `None`.
async fn index_sides(
    repo_root: &Path,
    path: &str,
    original: Option<&str>,
    mime_type: &str,
    stages: IndexStages,
    cancel_token: Option<BundleCancelToken>,
) -> Result<SourceControlImageDiff, AgentError> {
    let head_path = original.unwrap_or(path);
    let head_mime = mime_type_for_path(head_path);
    let head_spec = blob_spec("HEAD:", head_path);
    let before = match head_mime {
        Some(head_mime)
            if head_blob_exists(repo_root, &head_spec, cancel_token.clone()).await? =>
        {
            blob_side(repo_root, &head_spec, ImageDiffSource::Head, head_mime, cancel_token.clone())
                .await?
        }
        _ => None,
    };
    let after = if stages.staged {
        blob_side(repo_root, &blob_spec(":0:", path), ImageDiffSource::Index, mime_type, cancel_token).await?
    } else {
        None
    };
    Ok(SourceControlImageDiff { before, after })
}

/// Unstaged change: the staged snapshot against the file on disk. An
/// intent-to-add entry stages an empty blob, which `blob_side` reports as
/// `None`; a file deleted on disk is a `None` after side.
async fn worktree_sides(
    repo_root: &Path,
    path: &str,
    mime_type: &str,
    stages: IndexStages,
    cancel_token: Option<BundleCancelToken>,
) -> Result<SourceControlImageDiff, AgentError> {
    let before = if stages.staged {
        blob_side(repo_root, &blob_spec(":0:", path), ImageDiffSource::Index, mime_type, cancel_token).await?
    } else {
        None
    };
    let after = worktree_side(repo_root, path, mime_type).await?;
    Ok(SourceControlImageDiff { before, after })
}

/// `ls-files --stage -z` prints `<mode> <object> <stage>\t<path>` per NUL-
/// terminated record. Only the stage column is read; the path is already known
/// and non-UTF-8 names never reach here.
async fn index_stages(
    repo_root: &Path,
    path: &str,
    cancel_token: Option<BundleCancelToken>,
) -> Result<IndexStages, AgentError> {
    let call = GitCall::new(["ls-files", "--stage", "-z", "--"]).arg(path);
    let output = runner::run_read(repo_root, call, cancel_token).await?;
    let mut stages = IndexStages::default();
    for record in output.stdout.split(|byte| *byte == 0) {
        let Some(header) = record.split(|byte| *byte == b'\t').next() else {
            continue;
        };
        match String::from_utf8_lossy(header).split_whitespace().nth(2) {
            Some("0") => stages.staged = true,
            Some("2") => stages.ours = true,
            Some("3") => stages.theirs = true,
            _ => {}
        }
    }
    Ok(stages)
}
