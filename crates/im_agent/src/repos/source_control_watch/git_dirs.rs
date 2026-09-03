// Path: crates/im_agent/src/repos/source_control_watch/git_dirs.rs
// Description: Resolve a repo's git dir and common dir so linked worktrees stay watched

use im_bundle::git::{bytes_to_path, common_git_args, run_git, trim_line_ending};
use notify::RecursiveMode;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::logging::Logger;

const GIT_EXECUTABLE: &str = "git";
const RESOLVE_TIMEOUT: Duration = Duration::from_secs(5);
const STDOUT_LIMIT: usize = 1024 * 1024;
const REFS_DIR: &str = "refs";

/// A repository's own git dir and the dir shared with its linked worktrees.
/// They are the same path for an ordinary repo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GitDirs {
    pub(crate) git_dir: PathBuf,
    pub(crate) common_dir: PathBuf,
}

/// Extra `notify` registrations plus the git dirs the detector's metadata rule
/// applies to. Empty for a repo whose git dir lives inside the watched root.
#[derive(Debug, Default)]
pub(crate) struct ExternalGitWatches {
    pub(crate) watch_paths: Vec<(PathBuf, RecursiveMode)>,
    pub(crate) detector_dirs: Vec<PathBuf>,
}

/// Resolves the git dirs and turns them into extra watches, logging once and
/// degrading to "no metadata signal" when Git is missing, the root is not a
/// repository, or the probe times out.
pub(crate) async fn resolve_external_watches(
    root_path: &Path,
    logger: &Logger,
) -> ExternalGitWatches {
    match resolve_git_dirs(root_path).await {
        Ok(dirs) => external_watches(root_path, &dirs),
        Err(reason) => {
            logger.warn(
                "Source control watch has no git metadata signal",
                Some(serde_json::json!({
                    "rootPath": root_path.to_string_lossy(),
                    "reason": reason,
                })),
            );
            ExternalGitWatches::default()
        }
    }
}

pub(crate) async fn resolve_git_dirs(root_path: &Path) -> Result<GitDirs, String> {
    let repo_root = root_path.to_path_buf();
    let stdout = tokio::task::spawn_blocking(move || {
        let mut args = common_git_args();
        args.extend(
            ["rev-parse", "--git-dir", "--git-common-dir"]
                .into_iter()
                .map(OsString::from),
        );
        run_git(
            Path::new(GIT_EXECUTABLE),
            &repo_root,
            &args,
            STDOUT_LIMIT,
            RESOLVE_TIMEOUT,
            None,
        )
    })
    .await
    .map_err(|err| format!("git rev-parse task failed: {err}"))?
    .map_err(|err| format!("git rev-parse failed: {err}"))?
    .map(|output| output.stdout)
    .map_err(|failure| {
        format!(
            "git rev-parse failed: {:?} {}",
            failure.kind,
            failure.message()
        )
    })?;

    let mut lines = stdout
        .split(|byte| *byte == b'\n')
        .map(|line| trim_line_ending(line.to_vec()))
        .filter(|line| !line.is_empty());
    let git_dir = lines
        .next()
        .and_then(|line| bytes_to_path(&line))
        .ok_or_else(|| "git rev-parse returned no git dir".to_string())?;
    let common_dir = lines
        .next()
        .and_then(|line| bytes_to_path(&line))
        .unwrap_or_else(|| git_dir.clone());

    Ok(GitDirs {
        git_dir: absolutize(root_path, git_dir),
        common_dir: absolutize(root_path, common_dir),
    })
}

fn external_watches(root_path: &Path, dirs: &GitDirs) -> ExternalGitWatches {
    let root = canonical(root_path.to_path_buf());
    if dirs.git_dir.starts_with(&root) {
        return ExternalGitWatches::default();
    }

    let mut watches = ExternalGitWatches::default();
    push_watch(&mut watches, dirs.git_dir.clone(), RecursiveMode::Recursive);
    push_watch(
        &mut watches,
        dirs.common_dir.join(REFS_DIR),
        RecursiveMode::Recursive,
    );
    // Non-recursive: the common dir is watched for `packed-refs` and `HEAD`,
    // not for its object database.
    push_watch(
        &mut watches,
        dirs.common_dir.clone(),
        RecursiveMode::NonRecursive,
    );

    watches.detector_dirs = vec![dirs.git_dir.clone()];
    if dirs.common_dir != dirs.git_dir {
        watches.detector_dirs.push(dirs.common_dir.clone());
    }
    watches
}

fn push_watch(watches: &mut ExternalGitWatches, path: PathBuf, mode: RecursiveMode) {
    if watches
        .watch_paths
        .iter()
        .any(|(existing, _)| *existing == path)
    {
        return;
    }
    watches.watch_paths.push((path, mode));
}

fn absolutize(root_path: &Path, path: PathBuf) -> PathBuf {
    let joined = if path.is_absolute() {
        path
    } else {
        root_path.join(path)
    };
    canonical(joined)
}

fn canonical(path: PathBuf) -> PathBuf {
    std::fs::canonicalize(&path).unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::{external_watches, resolve_git_dirs};
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
    async fn plain_repo_resolves_dirs_inside_the_root() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        git(root, &["init", "-q"]);

        let dirs = resolve_git_dirs(root).await.expect("resolve git dirs");
        let canonical_root = std::fs::canonicalize(root).expect("canonical root");
        assert!(dirs.git_dir.starts_with(&canonical_root), "{dirs:?}");
        assert!(dirs.common_dir.starts_with(&canonical_root), "{dirs:?}");
        let watches = external_watches(root, &dirs);
        assert!(watches.watch_paths.is_empty(), "{watches:?}");
        assert!(watches.detector_dirs.is_empty(), "{watches:?}");
    }

    #[tokio::test]
    async fn linked_worktree_resolves_a_git_dir_outside_the_root() {
        let temp = tempfile::tempdir().expect("tempdir");
        let main = temp.path().join("main");
        let linked = temp.path().join("linked");
        std::fs::create_dir_all(&main).expect("create main repo dir");
        git(&main, &["init", "-q"]);
        std::fs::write(main.join("a.txt"), b"a").expect("seed file");
        git(&main, &["add", "a.txt"]);
        git(
            &main,
            &[
                "-c",
                "user.email=test@example.com",
                "-c",
                "user.name=Test",
                "commit",
                "-q",
                "-m",
                "seed",
            ],
        );
        git(
            &main,
            &["worktree", "add", "-q", linked.to_str().expect("utf8 path")],
        );

        let dirs = resolve_git_dirs(&linked).await.expect("resolve git dirs");
        let canonical_linked = std::fs::canonicalize(&linked).expect("canonical linked root");
        assert!(!dirs.git_dir.starts_with(&canonical_linked), "{dirs:?}");
        assert_ne!(dirs.git_dir, dirs.common_dir);

        let watches = external_watches(&linked, &dirs);
        assert_eq!(watches.watch_paths.len(), 3, "{watches:?}");
        assert_eq!(
            watches.detector_dirs,
            vec![dirs.git_dir.clone(), dirs.common_dir.clone()]
        );
    }

    #[tokio::test]
    async fn non_repository_root_reports_an_error() {
        let temp = tempfile::tempdir().expect("tempdir");
        assert!(resolve_git_dirs(temp.path()).await.is_err());
    }
}
