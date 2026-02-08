// Path: crates/im_host_agent/src/runtime/wsl_client_hello_cache.rs
// Description: Caches and fingerprints latest WSL clientHello payload for resilient bootstrap replay

use im_agent::error::AgentError;
use im_agent::protocol::ClientHelloCommand;

use super::host_runtime_helpers::client_hello_fingerprint;

#[derive(Clone, Debug)]
pub(super) struct CachedWslHello {
    pub command: ClientHelloCommand,
    pub fingerprint: String,
}

#[derive(Default, Debug)]
pub(super) struct WslClientHelloCache {
    latest: Option<CachedWslHello>,
    applied_fingerprint: Option<String>,
}

impl WslClientHelloCache {
    pub fn clear(&mut self) {
        self.latest = None;
        self.applied_fingerprint = None;
    }

    pub fn update_latest(
        &mut self,
        command: ClientHelloCommand,
    ) -> Result<CachedWslHello, AgentError> {
        let fingerprint = client_hello_fingerprint(&command)?;
        let cached = CachedWslHello {
            command,
            fingerprint,
        };
        self.latest = Some(cached.clone());
        Ok(cached)
    }

    pub fn pending(&self) -> Option<CachedWslHello> {
        let latest = self.latest.as_ref()?;
        if self.applied_fingerprint.as_deref() == Some(latest.fingerprint.as_str()) {
            return None;
        }
        Some(latest.clone())
    }

    pub fn mark_applied_if_latest(&mut self, fingerprint: &str) {
        if self
            .latest
            .as_ref()
            .map(|latest| latest.fingerprint.as_str())
            != Some(fingerprint)
        {
            return;
        }
        self.applied_fingerprint = Some(fingerprint.to_string());
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::WslClientHelloCache;
    use im_agent::protocol::ClientHelloCommand;

    fn hello(repo_id: &str, auto_stage: bool) -> ClientHelloCommand {
        ClientHelloCommand {
            config: json!({"repos": [{"repoId": repo_id, "root": {"kind": "wsl", "path": "/repo"}}]}),
            staging_host_root: "C:\\staging".to_string(),
            staging_wsl_root: Some("/mnt/c/staging".to_string()),
            auto_stage_on_change: Some(auto_stage),
        }
    }

    #[test]
    fn pending_until_applied() {
        let mut cache = WslClientHelloCache::default();
        let pending = cache
            .update_latest(hello("repo_a", false))
            .expect("cache latest");
        assert!(cache.pending().is_some());

        cache.mark_applied_if_latest(&pending.fingerprint);
        assert!(cache.pending().is_none());
    }

    #[test]
    fn updated_payload_becomes_pending_again() {
        let mut cache = WslClientHelloCache::default();
        let first = cache.update_latest(hello("repo_a", false)).expect("first");
        cache.mark_applied_if_latest(&first.fingerprint);
        assert!(cache.pending().is_none());

        let second = cache.update_latest(hello("repo_a", true)).expect("second");
        assert_ne!(first.fingerprint, second.fingerprint);
        assert!(cache.pending().is_some());
    }

    #[test]
    fn clear_resets_pending_state() {
        let mut cache = WslClientHelloCache::default();
        let pending = cache
            .update_latest(hello("repo_a", false))
            .expect("pending");
        cache.mark_applied_if_latest(&pending.fingerprint);
        cache.clear();
        assert!(cache.pending().is_none());
    }
}
