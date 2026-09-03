// Path: crates/im_agent/src/source_control/status/snapshot.rs
// Description: One reviewed-snapshot identity over branch, HEAD, index tree, and in-progress merge state

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::protocol::SourceControlStatus;

/// The sequencer files Git writes into its git dir while a merge, cherry-pick
/// or revert is being concluded. Their presence — and their contents, which
/// name the commits being brought in — change what `git commit` will record
/// from an otherwise identical index, so they belong to the identity a commit
/// is bound to.
const IN_PROGRESS_FILES: [&str; 3] = ["MERGE_HEAD", "CHERRY_PICK_HEAD", "REVERT_HEAD"];

/// The identity of everything the reviewed commit depends on: which ref the
/// commit will move, where that ref points now, what tree it would record, and
/// which in-progress operation it would conclude. Any of those moving means the
/// user is looking at a repository that no longer exists.
///
/// Empty means "no identity was captured", never "these components were all
/// empty": a torn index read (`index_tree_sha` empty) and an unreadable
/// sequencer file both produce it, and an empty id is refused by the commit
/// precondition rather than compared.
///
/// `git_dir` is the physical directory `rev-parse --absolute-git-dir` resolved
/// for this root, so a linked worktree is read at its own sequencer state and
/// not at the main checkout's.
pub(super) async fn capture_snapshot_id(git_dir: &Path, status: &SourceControlStatus) -> String {
    if status.index_tree_sha.is_empty() {
        return String::new();
    }
    let Some(in_progress) = read_in_progress(git_dir).await else {
        return String::new();
    };
    let branch = if status.detached {
        "detached"
    } else {
        status.branch.as_deref().unwrap_or_default()
    };
    let head = status.head_sha.as_deref().unwrap_or("unborn");
    let mut hasher = Sha256::new();
    let mut separated = false;
    let head_components = [branch.as_bytes(), head.as_bytes(), status.index_tree_sha.as_bytes()];
    for component in head_components.into_iter().chain(in_progress.iter().map(Vec::as_slice)) {
        if separated {
            hasher.update([0_u8]);
        }
        separated = true;
        hasher.update(component);
    }
    format!("{:x}", hasher.finalize())
}

/// The contents of every in-progress file, in `IN_PROGRESS_FILES` order, with
/// an absent file reading as empty. `None` means a read failed for any other
/// reason: the state is unknown, so no identity is claimed.
async fn read_in_progress(git_dir: &Path) -> Option<Vec<Vec<u8>>> {
    let git_dir = git_dir.to_path_buf();
    match tokio::task::spawn_blocking(move || read_in_progress_blocking(&git_dir)).await {
        Ok(Ok(contents)) => Some(contents),
        Ok(Err((path, error))) => {
            log_unreadable(&path.display().to_string(), &error.to_string());
            None
        }
        Err(error) => {
            log_unreadable("<join>", &error.to_string());
            None
        }
    }
}

fn read_in_progress_blocking(git_dir: &Path) -> Result<Vec<Vec<u8>>, (PathBuf, std::io::Error)> {
    let mut contents = Vec::with_capacity(IN_PROGRESS_FILES.len());
    for name in IN_PROGRESS_FILES {
        let path = git_dir.join(name);
        match std::fs::read(&path) {
            Ok(bytes) => contents.push(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => contents.push(Vec::new()),
            Err(error) => return Err((path, error)),
        }
    }
    Ok(contents)
}

fn log_unreadable(path: &str, error: &str) {
    eprintln!(
        "{{\"level\":\"warn\",\"msg\":\"source control snapshot identity could not read in-progress state\",\"path\":{path:?},\"error\":{error:?}}}"
    );
}

#[cfg(test)]
mod tests {
    use crate::protocol::{SourceControlOmitted, SourceControlStatus};

    use super::capture_snapshot_id;

    fn status() -> SourceControlStatus {
        SourceControlStatus {
            branch: Some("main".to_string()),
            head_sha: Some("aaaa".to_string()),
            detached: false,
            upstream: None,
            ahead: None,
            behind: None,
            index: Vec::new(),
            worktree: Vec::new(),
            conflicts: Vec::new(),
            committable: true,
            index_tree_sha: "tree".to_string(),
            snapshot_id: String::new(),
            mutation_in_progress: false,
            omitted: SourceControlOmitted::default(),
            truncated: false,
            captured_at_iso: "2026-09-03T00:00:00.000Z".to_string(),
        }
    }

    /// Every component moves the identity, and none of them can be traded for
    /// another: a branch named like a sha, or a sha that reads like a branch,
    /// still hash apart because the components are NUL-separated.
    #[tokio::test]
    async fn each_component_changes_the_identity() {
        let temp = tempfile::tempdir().expect("tempdir");
        let git_dir = temp.path();
        let base = capture_snapshot_id(git_dir, &status()).await;
        assert_eq!(base.len(), 64, "hex sha-256");

        for mutate in [
            (|status: &mut SourceControlStatus| status.branch = Some("other".to_string()))
                as fn(&mut SourceControlStatus),
            |status| status.detached = true,
            |status| status.head_sha = Some("bbbb".to_string()),
            |status| status.head_sha = None,
            |status| status.index_tree_sha = "other-tree".to_string(),
        ] {
            let mut moved = status();
            mutate(&mut moved);
            assert_ne!(capture_snapshot_id(git_dir, &moved).await, base);
        }

        // A boundary shift alone: "main" + "aaaa" must not collide with
        // "mai" + "naaaa".
        let mut shifted = status();
        shifted.branch = Some("mai".to_string());
        shifted.head_sha = Some("naaaa".to_string());
        assert_ne!(capture_snapshot_id(git_dir, &shifted).await, base);
    }

    /// A torn index read carries no identity at all, and never one that could
    /// compare equal to another torn read's.
    #[tokio::test]
    async fn a_torn_index_read_has_no_identity() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut torn = status();
        torn.index_tree_sha = String::new();
        assert_eq!(capture_snapshot_id(temp.path(), &torn).await, "");
    }

    /// An in-progress merge is part of the identity, and so is which commit it
    /// is merging: concluding a different merge from the same index is a
    /// different commit.
    #[tokio::test]
    async fn the_in_progress_state_is_part_of_the_identity() {
        let temp = tempfile::tempdir().expect("tempdir");
        let git_dir = temp.path();
        let clean = capture_snapshot_id(git_dir, &status()).await;

        std::fs::write(git_dir.join("MERGE_HEAD"), b"cccc\n").expect("MERGE_HEAD");
        let merging = capture_snapshot_id(git_dir, &status()).await;
        assert_ne!(merging, clean);

        std::fs::write(git_dir.join("MERGE_HEAD"), b"dddd\n").expect("MERGE_HEAD");
        assert_ne!(capture_snapshot_id(git_dir, &status()).await, merging);

        std::fs::remove_file(git_dir.join("MERGE_HEAD")).expect("remove MERGE_HEAD");
        std::fs::write(git_dir.join("REVERT_HEAD"), b"cccc\n").expect("REVERT_HEAD");
        assert_ne!(
            capture_snapshot_id(git_dir, &status()).await,
            merging,
            "the same commit in a different sequencer file is a different operation"
        );
    }

    /// A git dir that cannot be read at all yields no identity rather than one
    /// that silently ignores the state it could not see.
    #[cfg(unix)]
    #[tokio::test]
    async fn an_unreadable_in_progress_file_yields_no_identity() {
        let temp = tempfile::tempdir().expect("tempdir");
        // A directory where a file is expected: `read` fails with something
        // other than NotFound.
        std::fs::create_dir(temp.path().join("MERGE_HEAD")).expect("directory");
        assert_eq!(capture_snapshot_id(temp.path(), &status()).await, "");
    }
}
