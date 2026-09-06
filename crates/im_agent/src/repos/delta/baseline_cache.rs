// Path: crates/im_agent/src/repos/delta/baseline_cache.rs
// Description: Byte-bounded LRU of the text last served per path, per repo

use std::collections::HashMap;
use std::sync::Arc;

use super::MAX_DELTA_FILE_BYTES;

struct CacheEntry {
    text: Arc<str>,
    bytes: usize,
    /// Monotonic recency stamp; the smallest one is the least recently used.
    used: u64,
}

/// The baseline half of the delta invariant: the exact text this agent process
/// last published for a path. Never persisted - a restart legitimately falls
/// back to the index blob, and the card says so.
pub(crate) struct BaselineCache {
    budget_bytes: usize,
    bytes: usize,
    tick: u64,
    entries: HashMap<String, CacheEntry>,
}

impl BaselineCache {
    pub(crate) fn new(budget_bytes: usize) -> Self {
        Self {
            budget_bytes,
            bytes: 0,
            tick: 0,
            entries: HashMap::new(),
        }
    }

    /// Reads a baseline and refreshes its recency.
    pub(crate) fn get(&mut self, path: &str) -> Option<Arc<str>> {
        self.tick = self.tick.saturating_add(1);
        let tick = self.tick;
        let entry = self.entries.get_mut(path)?;
        entry.used = tick;
        Some(Arc::clone(&entry.text))
    }

    /// Stores a baseline, dropping least-recently-used entries until the repo is
    /// back inside its byte budget. Text over `MAX_DELTA_FILE_BYTES` is refused:
    /// such a path is `Opaque(tooLarge)` and has no baseline by definition.
    pub(crate) fn insert(&mut self, path: String, text: Arc<str>) {
        let bytes = text.len();
        if bytes as u64 > MAX_DELTA_FILE_BYTES || bytes > self.budget_bytes {
            self.remove(&path);
            return;
        }
        self.tick = self.tick.saturating_add(1);
        let used = self.tick;
        if let Some(previous) = self.entries.insert(path, CacheEntry { text, bytes, used }) {
            self.bytes = self.bytes.saturating_sub(previous.bytes);
        }
        self.bytes = self.bytes.saturating_add(bytes);
        self.evict_to_budget();
    }

    pub(crate) fn remove(&mut self, path: &str) -> Option<Arc<str>> {
        let entry = self.entries.remove(path)?;
        self.bytes = self.bytes.saturating_sub(entry.bytes);
        Some(entry.text)
    }

    /// Moves a baseline with its path so a rename keeps diffing against the
    /// content the reader last saw rather than falling back to the index.
    pub(crate) fn rename(&mut self, from: &str, to: &str) {
        let Some(text) = self.remove(from) else {
            self.remove(to);
            return;
        };
        self.insert(to.to_string(), text);
    }

    pub(crate) fn bytes(&self) -> usize {
        self.bytes
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    /// One O(n) scan per evicted entry. At steady state an insert evicts at most
    /// a handful, and inserts are already bounded by the burst budget.
    fn evict_to_budget(&mut self) {
        while self.bytes > self.budget_bytes {
            let Some(victim) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.used)
                .map(|(path, _)| path.clone())
            else {
                break;
            };
            if self.remove(&victim).is_none() {
                break;
            }
        }
    }
}
