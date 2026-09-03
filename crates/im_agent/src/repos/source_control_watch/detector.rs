// Path: crates/im_agent/src/repos/source_control_watch/detector.rs
// Description: Decide whether a raw watcher event can move `git status` for a repo

use globset::{Glob, GlobBuilder, GlobSet, GlobSetBuilder};
use notify::{Event, EventKind};
use std::path::{Path, PathBuf};

use crate::error::AgentError;

/// Directories that never hold tracked files. Deliberately minimal: the
/// watcher's `IgnoreMatcher` defaults (`**/.git/**`, `**/dist/**`, `**/*.log`,
/// `**/logs/**`, `**/build/**`) would hide files Git tracks, so this detector
/// owns its own matcher instead of reusing that one.
const STRUCTURAL_IGNORE_PATTERNS: &[&str] = &["**/node_modules/**", "**/target/**"];

/// Git dir entries whose content decides what `git status` prints. Everything
/// else under a git dir (objects, logs, hooks, info) is noise.
const GIT_METADATA_ENTRIES: &[&str] = &[
    "index",
    "HEAD",
    "ORIG_HEAD",
    "MERGE_HEAD",
    "FETCH_HEAD",
    "packed-refs",
];

const GIT_DIR_ENTRY: &str = ".git";
const GIT_DIR_PREFIX: &str = ".git/";
const REFS_PREFIX: &str = "refs/";
const LOCK_SUFFIX: &str = ".lock";

pub(crate) struct SourceControlChangeDetector {
    root_path: PathBuf,
    external_git_dirs: Vec<PathBuf>,
    ignore_set: GlobSet,
}

impl SourceControlChangeDetector {
    /// `external_git_dirs` are absolute git dirs living outside `root_path`
    /// (a linked worktree's git dir and its common dir); paths inside them are
    /// judged by the git-metadata rule only.
    pub(crate) fn new(
        root_path: &Path,
        external_git_dirs: Vec<PathBuf>,
        ignore_globs: &[String],
    ) -> Result<Self, AgentError> {
        let mut builder = GlobSetBuilder::new();
        for pattern in STRUCTURAL_IGNORE_PATTERNS {
            builder.add(build_glob(pattern)?);
        }
        for pattern in ignore_globs {
            builder.add(build_glob(pattern)?);
        }
        let ignore_set = builder
            .build()
            .map_err(|err| AgentError::new("INVALID_GLOB", err.to_string()))?;
        Ok(Self {
            root_path: root_path.to_path_buf(),
            external_git_dirs,
            ignore_set,
        })
    }

    pub(crate) fn affects(&self, event: &Event) -> bool {
        if !matches!(
            event.kind,
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
        ) {
            return false;
        }
        event.paths.iter().any(|path| self.affects_path(path))
    }

    fn affects_path(&self, path: &Path) -> bool {
        for git_dir in &self.external_git_dirs {
            if let Ok(relative) = path.strip_prefix(git_dir) {
                return is_git_metadata(&normalize(relative));
            }
        }

        let Ok(relative) = path.strip_prefix(&self.root_path) else {
            return false;
        };
        let relative = normalize(relative);
        if relative == GIT_DIR_ENTRY {
            // `.git` itself: `git init`, or the pointer file of a linked worktree.
            return true;
        }
        if let Some(entry) = relative.strip_prefix(GIT_DIR_PREFIX) {
            return is_git_metadata(entry);
        }
        if relative.is_empty() {
            return false;
        }
        !self.ignore_set.is_match(&relative)
    }
}

fn normalize(relative: &Path) -> String {
    relative
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
}

/// Git writes `index`, `HEAD` and refs through a lockfile rename, so the
/// `.lock` sibling is the same signal.
fn is_git_metadata(relative: &str) -> bool {
    let entry = relative.strip_suffix(LOCK_SUFFIX).unwrap_or(relative);
    GIT_METADATA_ENTRIES.contains(&entry) || entry.starts_with(REFS_PREFIX)
}

fn build_glob(pattern: &str) -> Result<Glob, AgentError> {
    GlobBuilder::new(pattern)
        .case_insensitive(true)
        .literal_separator(false)
        .backslash_escape(false)
        .build()
        .map_err(|err| AgentError::new("INVALID_GLOB", err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::SourceControlChangeDetector;
    use notify::event::{AccessKind, CreateKind, ModifyKind, RenameMode};
    use notify::{Event, EventKind};
    use std::path::{Path, PathBuf};

    const ROOT: &str = "/repo";
    const EXTERNAL_GIT_DIR: &str = "/main/.git/worktrees/feature";

    fn detector(ignore_globs: &[&str]) -> SourceControlChangeDetector {
        let globs: Vec<String> = ignore_globs.iter().map(|glob| glob.to_string()).collect();
        SourceControlChangeDetector::new(
            Path::new(ROOT),
            vec![PathBuf::from(EXTERNAL_GIT_DIR)],
            &globs,
        )
        .expect("detector builds")
    }

    fn modified(path: &str) -> Event {
        Event::new(EventKind::Modify(ModifyKind::Any)).add_path(PathBuf::from(path))
    }

    #[test]
    fn working_tree_changes_fire() {
        let detector = detector(&[]);
        for path in [
            "/repo/logs/app.log",
            "/repo/dist/index.js",
            "/repo/Cargo.lock",
            "/repo/build/out.bin",
            "/repo/src/main.rs",
        ] {
            assert!(detector.affects(&modified(path)), "expected fire: {path}");
        }
    }

    #[test]
    fn git_metadata_changes_fire() {
        let detector = detector(&[]);
        for path in [
            "/repo/.git/index",
            "/repo/.git/index.lock",
            "/repo/.git/HEAD",
            "/repo/.git/MERGE_HEAD",
            "/repo/.git/packed-refs",
            "/repo/.git/refs/heads/main",
            "/repo/.git/refs/heads/main.lock",
            "/repo/.git",
        ] {
            assert!(detector.affects(&modified(path)), "expected fire: {path}");
        }
    }

    #[test]
    fn git_dir_creation_fires() {
        let detector = detector(&[]);
        let event = Event::new(EventKind::Create(CreateKind::Folder)).add_path("/repo/.git".into());
        assert!(detector.affects(&event));
    }

    #[test]
    fn structural_and_git_noise_do_not_fire() {
        let detector = detector(&[]);
        for path in [
            "/repo/target/x.o",
            "/repo/node_modules/a/b.js",
            "/repo/.git/objects/ab/cd",
            "/repo/.git/logs/HEAD",
            "/repo/.git/hooks/pre-commit",
            "/elsewhere/file.rs",
        ] {
            assert!(!detector.affects(&modified(path)), "expected quiet: {path}");
        }
    }

    #[test]
    fn access_events_do_not_fire() {
        let detector = detector(&[]);
        let event =
            Event::new(EventKind::Access(AccessKind::Any)).add_path("/repo/src/main.rs".into());
        assert!(!detector.affects(&event));
    }

    #[test]
    fn rename_events_fire() {
        let detector = detector(&[]);
        let event = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
            .add_path("/repo/.git/index.lock".into())
            .add_path("/repo/.git/index".into());
        assert!(detector.affects(&event));
    }

    #[test]
    fn external_git_dir_applies_metadata_rule_only() {
        let detector = detector(&[]);
        assert!(detector.affects(&modified("/main/.git/worktrees/feature/HEAD")));
        assert!(detector.affects(&modified("/main/.git/worktrees/feature/refs/bisect/bad")));
        assert!(!detector.affects(&modified("/main/.git/worktrees/feature/objects/x")));
        assert!(!detector.affects(&modified("/main/.git/worktrees/feature/logs/HEAD")));
    }

    #[test]
    fn configured_ignore_globs_suppress() {
        let detector = detector(&["**/generated/**"]);
        assert!(!detector.affects(&modified("/repo/src/generated/api.rs")));
        assert!(detector.affects(&modified("/repo/src/api.rs")));
    }
}
