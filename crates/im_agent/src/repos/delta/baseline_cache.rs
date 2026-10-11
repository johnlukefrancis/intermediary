// Path: crates/im_agent/src/repos/delta/baseline_cache.rs
// Description: Byte-bounded LRU of the baseline (text or line fingerprints) last served per path, per repo

use std::collections::HashMap;

use super::{Baseline, MAX_DELTA_FILE_BYTES};

struct CacheEntry {
    baseline: Baseline,
    bytes: usize,
    /// Monotonic recency stamp; the smallest one is the least recently used.
    used: u64,
}

/// The text (or a large file's line fingerprints) this process last published per path.
/// Never persisted: a restart falls back to the index blob, and the card says so.
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
    pub(crate) fn get(&mut self, path: &str) -> Option<Baseline> {
        self.tick = self.tick.saturating_add(1);
        let tick = self.tick;
        let entry = self.entries.get_mut(path)?;
        entry.used = tick;
        Some(entry.baseline.clone())
    }

    /// Stores a baseline and evicts LRU entries back inside the byte budget. Text over
    /// `MAX_DELTA_FILE_BYTES` is refused; fingerprints are charged at 8 bytes per line.
    pub(crate) fn insert(&mut self, path: String, baseline: Baseline) {
        let bytes = baseline.bytes();
        let oversized_text =
            matches!(&baseline, Baseline::Text(text) if text.len() as u64 > MAX_DELTA_FILE_BYTES);
        if oversized_text || bytes > self.budget_bytes {
            self.remove(&path);
            return;
        }
        self.tick = self.tick.saturating_add(1);
        let used = self.tick;
        if let Some(previous) = self.entries.insert(
            path,
            CacheEntry {
                baseline,
                bytes,
                used,
            },
        ) {
            self.bytes = self.bytes.saturating_sub(previous.bytes);
        }
        self.bytes = self.bytes.saturating_add(bytes);
        self.evict_to_budget();
    }

    pub(crate) fn remove(&mut self, path: &str) -> Option<Baseline> {
        let entry = self.entries.remove(path)?;
        self.bytes = self.bytes.saturating_sub(entry.bytes);
        Some(entry.baseline)
    }

    /// Moves a baseline with its path. A source with no baseline (an atomic
    /// save's temp file) leaves the destination's own baseline in place.
    pub(crate) fn rename(&mut self, from: &str, to: &str) {
        if let Some(baseline) = self.remove(from) {
            self.insert(to.to_string(), baseline);
        }
    }

    /// Forgets every baseline: the worker lost track of which paths changed
    /// (the dropped-path record overflowed), so no cached text can be trusted
    /// to be what the reader last saw. Every next sighting says `VS INDEX`.
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.bytes = 0;
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
