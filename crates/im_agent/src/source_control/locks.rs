// Path: crates/im_agent/src/source_control/locks.rs
// Description: Per-repo mutation serialization for source-control actions

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// One async mutex per repo id, created on first use. Interior-mutable so the
/// host backend can reach it through `&self`. Callers clone the per-repo `Arc`
/// out (dropping this registry's guard immediately) and only then await it, so
/// no runtime lock is ever held across a Git process.
#[derive(Clone, Default)]
pub struct SourceControlLocks {
    locks: Arc<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
}

impl SourceControlLocks {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn lock_for(&self, repo_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        let mut locks = match self.locks.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        Arc::clone(
            locks
                .entry(repo_id.to_string())
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::SourceControlLocks;

    #[tokio::test]
    async fn same_repo_shares_one_lock_and_serializes() {
        let locks = SourceControlLocks::new();
        let first = locks.lock_for("repo");
        let second = locks.lock_for("repo");
        assert!(std::sync::Arc::ptr_eq(&first, &second));
        let guard = first.lock().await;
        assert!(second.try_lock().is_err());
        drop(guard);
        assert!(second.try_lock().is_ok());
    }

    #[test]
    fn different_repos_do_not_share_a_lock() {
        let locks = SourceControlLocks::new();
        assert!(!std::sync::Arc::ptr_eq(&locks.lock_for("a"), &locks.lock_for("b")));
    }
}
