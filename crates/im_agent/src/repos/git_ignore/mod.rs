// Path: crates/im_agent/src/repos/git_ignore/mod.rs
// Description: Watcher-side Git ignore filter: suppresses ignored untracked paths and reloads rules when ignore files change

#[cfg(test)]
mod git_ignore_tests;
mod loader;
mod rules;
#[cfg(test)]
mod rules_tests;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};

use notify::Event;

use crate::logging::Logger;
use crate::repos::source_control_watch::TrackedPathSet;
use loader::{load_rules, LoadedRules};

const GITIGNORE_NAME: &str = ".gitignore";

#[derive(Default)]
struct ReloadState {
    running: bool,
    rerun: bool,
    logged_failure: bool,
}

struct Inner {
    root_path: PathBuf,
    tracked: TrackedPathSet,
    logger: Logger,
    loaded: RwLock<Option<LoadedRules>>,
    seen_gitignores: Mutex<BTreeSet<String>>,
    reload: Mutex<ReloadState>,
}

/// Without loaded rules nothing is suppressed; a failed reload keeps the last
/// good rules. Clones share one state.
#[derive(Clone)]
pub(crate) struct GitIgnoreFilter {
    inner: Arc<Inner>,
}

impl GitIgnoreFilter {
    pub(crate) fn new(root_path: PathBuf, tracked: TrackedPathSet, logger: Logger) -> Self {
        Self {
            inner: Arc::new(Inner {
                root_path,
                tracked,
                logger,
                loaded: RwLock::new(None),
                seen_gitignores: Mutex::new(BTreeSet::new()),
                reload: Mutex::new(ReloadState::default()),
            }),
        }
    }

    /// Git semantics: ignored and untracked; a tracked path always counts.
    pub(crate) fn is_suppressed(&self, relative: &str) -> bool {
        let ignored = read_lock(&self.inner.loaded)
            .as_ref()
            .is_some_and(|loaded| loaded.is_ignored(relative, false));
        ignored && !self.inner.tracked.contains(relative)
    }

    /// Schedules a reload when an ignore file Git would read changes.
    pub(crate) fn note_event(&self, event: &Event) {
        let mut affected = false;
        for path in &event.paths {
            affected |= self.is_exclude_file(path) || self.note_gitignore(path);
        }
        if affected {
            self.schedule_reload();
        }
    }

    /// Loads now; startup awaits this so the first event is already filtered.
    pub(crate) async fn load_now(&self) {
        let extras: Vec<String> = lock(&self.inner.seen_gitignores).iter().cloned().collect();
        match load_rules(&self.inner.root_path, extras).await {
            Ok(loaded) => {
                *write_lock(&self.inner.loaded) = Some(loaded);
                lock(&self.inner.reload).logged_failure = false;
            }
            Err(reason) => {
                let first = !std::mem::replace(&mut lock(&self.inner.reload).logged_failure, true);
                if first {
                    self.inner.logger.warn(
                        "Git ignore rules failed to load",
                        Some(serde_json::json!({
                            "rootPath": self.inner.root_path.to_string_lossy(),
                            "reason": reason,
                        })),
                    );
                }
            }
        }
    }

    fn is_exclude_file(&self, path: &Path) -> bool {
        read_lock(&self.inner.loaded)
            .as_ref()
            .is_some_and(|loaded| loaded.exclude_path == path)
    }

    /// Git never reads a `.gitignore` inside an excluded directory.
    fn note_gitignore(&self, path: &Path) -> bool {
        if path.file_name().is_none_or(|name| name != GITIGNORE_NAME) {
            return false;
        }
        let Ok(relative) = path.strip_prefix(&self.inner.root_path) else {
            return false;
        };
        let relative = relative.to_string_lossy().replace('\\', "/");
        let dir = relative.rsplit_once('/').map_or("", |(dir, _)| dir);
        if dir == ".git" || dir.starts_with(".git/") {
            return false;
        }
        let dir_excluded = read_lock(&self.inner.loaded)
            .as_ref()
            .is_some_and(|loaded| loaded.is_ignored(dir, true));
        if dir_excluded && !self.inner.tracked.contains(&relative) {
            return false;
        }
        lock(&self.inner.seen_gitignores).insert(relative);
        true
    }

    /// One load in flight; changes during it fold into exactly one rerun.
    fn schedule_reload(&self) {
        {
            let mut state = lock(&self.inner.reload);
            if state.running {
                state.rerun = true;
                return;
            }
            state.running = true;
        }
        let filter = self.clone();
        tokio::spawn(async move {
            loop {
                filter.load_now().await;
                let mut state = lock(&filter.inner.reload);
                if !std::mem::take(&mut state.rerun) {
                    state.running = false;
                    break;
                }
            }
        });
    }
}

// Nothing awaits under these guards; recover a poisoned lock rather than
// take the watcher down (ADR-008).
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|err| err.into_inner())
}

fn read_lock<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|err| err.into_inner())
}

fn write_lock<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|err| err.into_inner())
}
