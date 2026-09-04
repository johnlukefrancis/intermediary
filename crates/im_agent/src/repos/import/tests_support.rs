// Path: crates/im_agent/src/repos/import/tests_support.rs
// Description: Shared fixtures for the import tests: a worktree, an external source, and one call

use std::fs;
use std::path::Path;

use tempfile::{tempdir, TempDir};

use crate::error::AgentError;
use crate::protocol::{ImportConflictPolicy, ImportedFile};
use crate::staging::{StageFileCancelToken, StagingRootKind};

use super::import_files;

pub(super) fn worktree() -> TempDir {
    let root = tempdir().expect("temp repo");
    fs::create_dir_all(root.path().join("app")).expect("app dir");
    root
}

pub(super) fn outside_file(dir: &Path, name: &str, contents: &str) -> String {
    let path = dir.join(name);
    fs::write(&path, contents).expect("write source");
    path.to_string_lossy().to_string()
}

pub(super) async fn import(
    root: &Path,
    directory: &str,
    sources: &[String],
    policy: ImportConflictPolicy,
) -> Result<Vec<ImportedFile>, AgentError> {
    import_files(
        root,
        directory,
        sources,
        &policy,
        StagingRootKind::Host,
        &StageFileCancelToken::new(),
    )
    .await
}
