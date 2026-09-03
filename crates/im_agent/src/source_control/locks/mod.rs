// Path: crates/im_agent/src/source_control/locks/mod.rs
// Description: Mutation serialization keyed by the physical Git directory, plus the drain gate

//! A mutation lock protects one physical index, not one UI label. Two
//! configured roots may name the same worktree (a repository root and a
//! subdirectory below it), and they must serialize against each other; a linked
//! worktree has its own index and must not. The key is therefore
//! `git rev-parse --absolute-git-dir`, resolved once per configured root and
//! cached for the process lifetime — normally by the status read, which pays
//! for that answer anyway and hands it back through `remember_git_dir`, so a
//! mutation resolves it itself only for a root nothing has read yet.
//!
//! Draining is the shutdown gate: once set, admission stops (`AGENT_DRAINING`,
//! proven `notApplied`) while reads keep being served, and `wait_idle` reports
//! whether the mutations still running finished inside their budget.

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use im_bundle::git::{bytes_to_path, trim_line_ending};

use crate::error::{AgentError, MutationEffect};

use crate::source_control::runner::{self, GitCall};

const IDLE_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Held for exactly as long as one mutation runs; the lock is released when it
/// is dropped, including on an early return or a panic.
pub struct MutationGuard {
    _guard: tokio::sync::OwnedMutexGuard<()>,
}

#[derive(Clone, Default)]
pub struct SourceControlLocks {
    inner: Arc<Registry>,
}

#[derive(Default)]
struct Registry {
    state: Mutex<RegistryState>,
    draining: AtomicBool,
}

#[derive(Default)]
struct RegistryState {
    git_dir_by_root: HashMap<PathBuf, PathBuf>,
    lock_by_git_dir: HashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>,
    quarantine_swept: HashSet<PathBuf>,
    live_discard_ops: HashSet<String>,
}

/// One discard operation, live for exactly as long as it may still create or
/// write quarantine directories. The startup sweep skips every directory named
/// after a live operation, so a discard running under one configured root can
/// never have its in-flight directory removed by the first status read of a
/// sibling root over the same git dir. Released on every exit path, including
/// a panic, because the registration is a `Drop`.
pub(super) struct DiscardOpGuard {
    locks: SourceControlLocks,
    op_id: String,
}

impl Drop for DiscardOpGuard {
    fn drop(&mut self) {
        self.locks.state().live_discard_ops.remove(&self.op_id);
    }
}

impl SourceControlLocks {
    pub fn new() -> Self {
        Self::default()
    }

    /// Admits one mutation for the worktree that owns `repo_root`. The
    /// registry's own std mutex is never held across an await, so resolution
    /// and queueing never block another repo's admission.
    pub async fn acquire(&self, repo_root: &Path) -> Result<MutationGuard, AgentError> {
        self.admission()?;
        let git_dir = self.git_dir(repo_root).await?;
        let lock = self.lock_for(&git_dir);
        self.admission()?;
        let guard = lock.lock_owned().await;
        // Draining can begin while this mutation waits its turn; a mutation
        // that has not started yet is refused with its effect proven.
        self.admission()?;
        Ok(MutationGuard { _guard: guard })
    }

    /// Whether a mutation holds this physical index right now. The caller
    /// supplies the git dir because it has already resolved it: a status read
    /// must not spawn Git to answer this, and asking by configured root would
    /// miss a sibling root over the same worktree that this process has never
    /// mutated through.
    pub fn is_busy_for_git_dir(&self, git_dir: &Path) -> bool {
        self.state()
            .lock_by_git_dir
            .get(git_dir)
            .is_some_and(|lock| lock.try_lock().is_err())
    }

    /// Records a git dir a read has already resolved, so the next mutation on
    /// this root does not spawn its own `rev-parse`. Both sides key the cache
    /// by the configured root exactly as given and the lock by Git's own
    /// `--absolute-git-dir` answer, so a warmed entry and one `acquire`
    /// resolves for itself are the same entry.
    pub fn remember_git_dir(&self, repo_root: &Path, git_dir: &Path) {
        self.state()
            .git_dir_by_root
            .insert(repo_root.to_path_buf(), git_dir.to_path_buf());
    }

    /// Whether this is the first time this process has been asked about
    /// `git_dir`'s discard quarantine directory: `true` exactly once per git
    /// dir per process, so a status read can trigger the bounded startup sweep
    /// (`discard::quarantine::sweep_stale_quarantine`) at most once instead of
    /// on every read.
    pub(super) fn mark_quarantine_swept(&self, git_dir: &Path) -> bool {
        self.state().quarantine_swept.insert(git_dir.to_path_buf())
    }

    /// Registers a discard operation before it creates its first quarantine
    /// directory. The sweep asks about the id again at the moment it is about
    /// to remove a directory, so a registration that lands before the
    /// directory exists is enough: any directory the sweep can see was created
    /// after its operation was registered.
    pub(super) fn register_discard_op(&self, op_id: &str) -> DiscardOpGuard {
        self.state().live_discard_ops.insert(op_id.to_string());
        DiscardOpGuard {
            locks: self.clone(),
            op_id: op_id.to_string(),
        }
    }

    /// Whether a quarantine directory belongs to a discard that is still
    /// running. Directories are named `<opId>-<targetIndex>`, so the operation
    /// owning one is read off the front of its name.
    pub(super) fn owns_live_discard(&self, directory_name: &str) -> bool {
        self.state().live_discard_ops.iter().any(|op_id| {
            directory_name
                .strip_prefix(op_id.as_str())
                .is_some_and(|target_index| target_index.starts_with('-'))
        })
    }

    pub fn set_draining(&self) {
        self.inner.draining.store(true, Ordering::SeqCst);
    }

    pub fn is_draining(&self) -> bool {
        self.inner.draining.load(Ordering::SeqCst)
    }

    /// Polls until no worktree lock is held, or the budget expires. Returns
    /// whether the agent reached idle.
    pub async fn wait_idle(&self, timeout: Duration) -> bool {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if !self.any_busy() {
                return true;
            }
            if tokio::time::Instant::now() >= deadline {
                return false;
            }
            tokio::time::sleep(IDLE_POLL_INTERVAL).await;
        }
    }

    /// How many worktree locks are held right now. This is the residue a
    /// shutdown reports when the drain budget expires: only the registry knows
    /// how many mutations are still running, and no caller can count them from
    /// `is_busy_for_git_dir` without a git dir per lock.
    pub fn busy_count(&self) -> u32 {
        let held = self
            .state()
            .lock_by_git_dir
            .values()
            .filter(|lock| lock.try_lock().is_err())
            .count();
        u32::try_from(held).unwrap_or(u32::MAX)
    }

    fn admission(&self) -> Result<(), AgentError> {
        if self.is_draining() {
            return Err(AgentError::new(
                "AGENT_DRAINING",
                "The agent is shutting down and is not starting new Git mutations",
            )
            .with_effect(MutationEffect::NotApplied));
        }
        Ok(())
    }

    fn any_busy(&self) -> bool {
        self.busy_count() > 0
    }

    async fn git_dir(&self, repo_root: &Path) -> Result<PathBuf, AgentError> {
        if let Some(cached) = self.state().git_dir_by_root.get(repo_root).cloned() {
            return Ok(cached);
        }
        let git_dir = resolve_git_dir(repo_root).await?;
        self.state()
            .git_dir_by_root
            .insert(repo_root.to_path_buf(), git_dir.clone());
        Ok(git_dir)
    }

    fn lock_for(&self, git_dir: &Path) -> Arc<tokio::sync::Mutex<()>> {
        Arc::clone(
            self.state()
                .lock_by_git_dir
                .entry(git_dir.to_path_buf())
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))),
        )
    }

    fn state(&self) -> std::sync::MutexGuard<'_, RegistryState> {
        match self.inner.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

async fn resolve_git_dir(repo_root: &Path) -> Result<PathBuf, AgentError> {
    let call = GitCall::new(["rev-parse", "--absolute-git-dir"]);
    let output = runner::run_read(repo_root, call, None).await?;
    bytes_to_path(&trim_line_ending(output.stdout)).ok_or_else(|| {
        AgentError::new(
            "GIT_NOT_REPOSITORY",
            format!(
                "Git reported no git directory for {}",
                repo_root.display()
            ),
        )
        .with_effect(MutationEffect::NotApplied)
    })
}
