// Path: crates/im_agent/src/repos/source_control_watch/tracked_set.rs
// Description: Tracked-path authority loaded from `git ls-files`, shared between the detector and its reloader

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use im_bundle::git::{bytes_to_path, common_git_args, run_git};

const GIT_EXECUTABLE: &str = "git";
const LS_FILES_TIMEOUT: Duration = Duration::from_secs(10);
const LS_FILES_STDOUT_LIMIT: usize = 64 * 1024 * 1024;

/// The set of repo-relative, slash-separated paths Git currently tracks.
/// Cloning shares the same backing set (cheap `Arc` clone): the detector
/// reads it on every event, the reloader replaces it wholesale once a load
/// completes, and a failed load leaves whatever was loaded before untouched.
#[derive(Clone)]
pub(crate) struct TrackedPathSet {
    paths: Arc<RwLock<HashSet<String>>>,
}

impl TrackedPathSet {
    pub(crate) fn empty() -> Self {
        Self {
            paths: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// `relative` is a repo-relative path with `/` separators, the same form
    /// `git ls-files` prints and the detector normalizes worktree events to.
    pub(crate) fn contains(&self, relative: &str) -> bool {
        self.lock_read().contains(relative)
    }

    pub(crate) fn store(&self, next: HashSet<String>) {
        *self.lock_write() = next;
    }

    fn lock_read(&self) -> std::sync::RwLockReadGuard<'_, HashSet<String>> {
        // No await happens under this guard; recover rather than panic on a
        // poisoned lock so a prior panic elsewhere cannot take the watcher
        // down (ADR-008).
        self.paths.read().unwrap_or_else(|err| err.into_inner())
    }

    fn lock_write(&self) -> std::sync::RwLockWriteGuard<'_, HashSet<String>> {
        self.paths.write().unwrap_or_else(|err| err.into_inner())
    }
}

/// Runs `git ls-files -z` on `spawn_blocking` (ADR-009, never on the async
/// runtime) and parses the NUL-separated, byte-exact output into
/// repo-relative slash paths. Errs with a human-readable reason when the root
/// is not a repository, Git is missing, or the probe times out; the caller
/// keeps whatever set it already had.
pub(crate) async fn load_tracked_paths(repo_root: &Path) -> Result<HashSet<String>, String> {
    let root = repo_root.to_path_buf();
    let stdout = tokio::task::spawn_blocking(move || {
        let mut args = common_git_args();
        args.extend(["ls-files", "-z"].into_iter().map(OsString::from));
        run_git(
            Path::new(GIT_EXECUTABLE),
            &root,
            &args,
            LS_FILES_STDOUT_LIMIT,
            LS_FILES_TIMEOUT,
            None,
        )
    })
    .await
    .map_err(|err| format!("git ls-files task failed: {err}"))?
    .map_err(|err| format!("git ls-files failed: {err}"))?
    .map(|output| output.stdout)
    .map_err(|failure| {
        format!(
            "git ls-files failed: {:?} {}",
            failure.kind,
            failure.message()
        )
    })?;

    Ok(parse_tracked_paths(&stdout))
}

fn parse_tracked_paths(stdout: &[u8]) -> HashSet<String> {
    stdout
        .split(|byte| *byte == 0)
        .filter(|chunk| !chunk.is_empty())
        .filter_map(bytes_to_path)
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{load_tracked_paths, TrackedPathSet};
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
    async fn load_reports_every_tracked_path() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        git(root, &["init", "-q"]);
        std::fs::create_dir_all(root.join("target")).expect("create target dir");
        std::fs::write(root.join("target/keep.rs"), b"x").expect("seed file");
        std::fs::write(root.join("target/other.rs"), b"y").expect("seed file");
        git(root, &["add", "target/keep.rs"]);

        let paths = load_tracked_paths(root).await.expect("load tracked paths");
        assert!(paths.contains("target/keep.rs"), "{paths:?}");
        assert!(!paths.contains("target/other.rs"), "{paths:?}");
    }

    #[tokio::test]
    async fn load_fails_on_a_non_repository_root() {
        let temp = tempfile::tempdir().expect("tempdir");
        assert!(load_tracked_paths(temp.path()).await.is_err());
    }

    #[test]
    fn store_replaces_the_whole_set() {
        let tracked = TrackedPathSet::empty();
        assert!(!tracked.contains("a.txt"));
        tracked.store(["a.txt".to_string()].into_iter().collect());
        assert!(tracked.contains("a.txt"));
        tracked.store(["b.txt".to_string()].into_iter().collect());
        assert!(!tracked.contains("a.txt"), "store replaces, not merges");
        assert!(tracked.contains("b.txt"));
    }
}
