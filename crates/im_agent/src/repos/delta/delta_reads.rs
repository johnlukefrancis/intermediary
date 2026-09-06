// Path: crates/im_agent/src/repos/delta/delta_reads.rs
// Description: The two reads a text delta depends on - index blob and settled worktree read - behind one seam the resolver is tested through

use std::path::Path;

use futures_util::future::BoxFuture;
use im_bundle::cancel::BundleCancelToken;

use crate::error::AgentError;
use crate::source_control::read_index_blob;

use super::{read_settled, ReadOutcome};

/// Where the resolver's bytes come from. Production reads Git and the disk;
/// a test substitutes recorders so the ORDER of the two reads - index blob
/// first, worktree second - is provable without a repository.
pub(super) trait ReadSources: Send + Sync {
    /// The stage-0 index text of `rel` under `root`, or `None` when the index
    /// has nothing usable. Runs on the async runtime (it spawns `git show`).
    fn index_text<'a>(
        &'a self,
        root: &'a Path,
        rel: &'a str,
        cancel: BundleCancelToken,
    ) -> BoxFuture<'a, Result<Option<String>, AgentError>>;

    /// The settled worktree read. Blocking; the resolver runs it inside
    /// `spawn_blocking` under an owned read permit.
    fn settled_read(
        &self,
        abs_path: &Path,
        expect_nonempty: bool,
        accept_moving: bool,
    ) -> ReadOutcome;
}

/// The production sources: `git show :0:./<rel>` and `read_settled`.
pub(super) struct DiskReads;

impl ReadSources for DiskReads {
    fn index_text<'a>(
        &'a self,
        root: &'a Path,
        rel: &'a str,
        cancel: BundleCancelToken,
    ) -> BoxFuture<'a, Result<Option<String>, AgentError>> {
        Box::pin(read_index_blob(root, rel, Some(cancel)))
    }

    fn settled_read(
        &self,
        abs_path: &Path,
        expect_nonempty: bool,
        accept_moving: bool,
    ) -> ReadOutcome {
        read_settled(abs_path, expect_nonempty, accept_moving)
    }
}
