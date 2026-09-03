// Path: crates/im_agent/src/repos/source_control_watch/detector.rs
// Description: Decide whether a raw watcher event can move `git status` for a repo

use globset::{Glob, GlobBuilder, GlobSet, GlobSetBuilder};
use notify::{Event, EventKind};
use std::path::{Path, PathBuf};

use crate::error::AgentError;

use super::tracked_set::TrackedPathSet;

/// Directories the detector treats as noise by default. Not a claim that
/// Git can never track a file under them (it can: `docs/known_issues.md`
/// records a real `target/` source directory) — `tracked` overrides this for
/// any path Git actually tracks, so this list only has to be a reasonable
/// default for the untracked case.
const STRUCTURAL_IGNORE_PATTERNS: &[&str] = &["**/node_modules/**", "**/target/**"];

/// Git dir entries whose content decides what `git status` prints. Everything
/// else under a git dir (objects, logs, hooks) is noise.
const GIT_METADATA_ENTRIES: &[&str] = &[
    "index",
    "HEAD",
    "ORIG_HEAD",
    "MERGE_HEAD",
    "FETCH_HEAD",
    "packed-refs",
    "config",
    "info/exclude",
];

const GIT_DIR_ENTRY: &str = ".git";
const GIT_DIR_PREFIX: &str = ".git/";
const REFS_PREFIX: &str = "refs/";
const WORKTREES_PREFIX: &str = "worktrees/";
const CONFIG_SUFFIX: &str = "/config";
const LOCK_SUFFIX: &str = ".lock";
const INDEX_ENTRY: &str = "index";

pub(crate) struct SourceControlChangeDetector {
    root_path: PathBuf,
    external_git_dirs: Vec<PathBuf>,
    ignore_set: GlobSet,
    tracked: TrackedPathSet,
}

/// Where a path lands relative to the repo this detector watches.
enum Location {
    /// Inside a git dir living outside `root_path` (a linked worktree's own
    /// git dir, or the common dir it shares with the main worktree): judged
    /// by the metadata rule only. Carries the dir-relative entry.
    ExternalGitDir(String),
    /// Under `root_path`'s own `.git/`. Carries the dir-relative entry.
    GitDir(String),
    /// `root_path/.git` itself: `git init`, or a linked worktree's pointer
    /// file.
    GitDirRoot,
    /// An ordinary worktree path. Carries the repo-relative, slash-separated
    /// path.
    Worktree(String),
    Outside,
}

impl SourceControlChangeDetector {
    /// `external_git_dirs` are absolute git dirs living outside `root_path`
    /// (a linked worktree's git dir and its common dir); paths inside them are
    /// judged by the git-metadata rule only. `tracked` is the repo's
    /// `git ls-files` authority, shared with the watch's reloader — a
    /// worktree event for a path it holds always emits, even under
    /// `ignore_globs`.
    pub(crate) fn new(
        root_path: &Path,
        external_git_dirs: Vec<PathBuf>,
        ignore_globs: &[String],
        tracked: TrackedPathSet,
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
            tracked,
        })
    }

    pub(crate) fn affects(&self, event: &Event) -> bool {
        if !is_relevant_kind(event) {
            return false;
        }
        event.paths.iter().any(|path| self.affects_path(path))
    }

    /// True when this event touches `.git/index` (or the lockfile Git
    /// renames it through) — the signal that the tracked-path set is stale.
    pub(crate) fn is_index_change(&self, event: &Event) -> bool {
        if !is_relevant_kind(event) {
            return false;
        }
        event.paths.iter().any(|path| match self.locate(path) {
            Location::ExternalGitDir(entry) | Location::GitDir(entry) => is_index_entry(&entry),
            Location::GitDirRoot | Location::Worktree(_) | Location::Outside => false,
        })
    }

    fn affects_path(&self, path: &Path) -> bool {
        match self.locate(path) {
            Location::ExternalGitDir(entry) | Location::GitDir(entry) => is_git_metadata(&entry),
            Location::GitDirRoot => true,
            Location::Worktree(relative) => {
                self.tracked.contains(&relative) || !self.ignore_set.is_match(&relative)
            }
            Location::Outside => false,
        }
    }

    fn locate(&self, path: &Path) -> Location {
        for git_dir in &self.external_git_dirs {
            if let Ok(relative) = path.strip_prefix(git_dir) {
                return Location::ExternalGitDir(normalize(relative));
            }
        }

        let Ok(relative) = path.strip_prefix(&self.root_path) else {
            return Location::Outside;
        };
        let relative = normalize(relative);
        if relative == GIT_DIR_ENTRY {
            return Location::GitDirRoot;
        }
        if let Some(entry) = relative.strip_prefix(GIT_DIR_PREFIX) {
            return Location::GitDir(entry.to_string());
        }
        if relative.is_empty() {
            return Location::Outside;
        }
        Location::Worktree(relative)
    }
}

fn is_relevant_kind(event: &Event) -> bool {
    matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

fn normalize(relative: &Path) -> String {
    relative
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
}

/// Git writes `index`, `HEAD` and refs through a lockfile rename, so the
/// `.lock` sibling is the same signal.
fn is_git_metadata(relative: &str) -> bool {
    let entry = strip_lock(relative);
    GIT_METADATA_ENTRIES.contains(&entry) || entry.starts_with(REFS_PREFIX) || is_worktree_config(entry)
}

fn is_index_entry(relative: &str) -> bool {
    strip_lock(relative) == INDEX_ENTRY
}

fn strip_lock(relative: &str) -> &str {
    relative.strip_suffix(LOCK_SUFFIX).unwrap_or(relative)
}

/// `.git/worktrees/<name>/config` — a linked worktree's config extension
/// (`extensions.worktreeConfig`), one segment deep under `worktrees/`.
fn is_worktree_config(entry: &str) -> bool {
    let Some(rest) = entry.strip_prefix(WORKTREES_PREFIX) else {
        return false;
    };
    let Some(name) = rest.strip_suffix(CONFIG_SUFFIX) else {
        return false;
    };
    !name.is_empty() && !name.contains('/')
}

fn build_glob(pattern: &str) -> Result<Glob, AgentError> {
    GlobBuilder::new(pattern)
        .case_insensitive(true)
        .literal_separator(false)
        .backslash_escape(false)
        .build()
        .map_err(|err| AgentError::new("INVALID_GLOB", err.to_string()))
}

