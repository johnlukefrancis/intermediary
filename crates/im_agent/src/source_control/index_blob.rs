// Path: crates/im_agent/src/source_control/index_blob.rs
// Description: Reads one path's stage-0 index blob (`git show :0:./<rel>`) as bounded UTF-8 text for the delta baseline

use std::path::Path;

use im_bundle::cancel::BundleCancelToken;

use crate::error::AgentError;
use crate::repos::delta::INDEX_BLOB_TIMEOUT;

use super::paths::normalize_path;
use super::runner::{self, GitCall};

/// Exit status Git uses when the path has no stage-0 entry (untracked,
/// deleted from the index, or held only at conflict stages).
const NOT_IN_INDEX_EXIT: i32 = 128;

/// Root-relative (`./`) stage-0 text, `Ok(None)` when absent, over `limit` bytes or not
/// UTF-8 text; errors are Git failures, including the child killed when `cancel` fires.
pub(crate) async fn read_index_blob(
    repo_root: &Path,
    rel: &str,
    limit: u64,
    cancel: Option<BundleCancelToken>,
) -> Result<Option<String>, AgentError> {
    let path = normalize_path(rel)?;
    let call = GitCall::new(["show"])
        .arg(format!(":0:./{path}"))
        .stdout_limit(usize::try_from(limit).unwrap_or(usize::MAX))
        .timeout(INDEX_BLOB_TIMEOUT)
        .accept_exit_codes(&[NOT_IN_INDEX_EXIT]);
    let output = runner::run_read(repo_root, call, cancel).await?;
    if output.exit_code != 0 || output.stdout_truncated || output.stdout.contains(&0) {
        return Ok(None);
    }
    Ok(String::from_utf8(output.stdout).ok())
}

#[cfg(test)]
mod tests {
    use super::read_index_blob;
    use std::path::Path;
    use std::process::Command;

    fn git(cwd: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .status()
            .expect("run git");
        assert!(status.success(), "git command failed: {args:?}");
    }

    #[tokio::test]
    async fn index_blob_is_text_when_staged_and_none_otherwise() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        git(root, &["init", "-q"]);
        std::fs::create_dir_all(root.join("src")).expect("mkdir");
        std::fs::write(root.join("src/a.rs"), "fn a() {}\n").expect("write");
        std::fs::write(root.join("src/bin.dat"), [0u8, 1, 2]).expect("write");
        git(root, &["add", "src/a.rs", "src/bin.dat"]);
        std::fs::write(root.join("src/a.rs"), "changed\n").expect("overwrite worktree");

        let staged = read_index_blob(root, "src/a.rs", 4096, None)
            .await
            .expect("git ran");
        assert_eq!(
            staged.as_deref(),
            Some("fn a() {}\n"),
            "index, not worktree"
        );
        assert_eq!(
            read_index_blob(root, "src/bin.dat", 4096, None)
                .await
                .expect("git ran"),
            None
        );
        assert_eq!(
            read_index_blob(root, "src/missing.rs", 4096, None)
                .await
                .expect("git ran"),
            None
        );
    }
}
