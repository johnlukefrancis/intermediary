// Path: crates/im_agent/src/repos/delta/baseline.rs
// Description: One retained baseline - full text for ordinary files, 64-bit line fingerprints for large ones

use std::hash::Hasher;
use std::sync::Arc;

use super::{MAX_DELTA_FILE_BYTES, MAX_FINGERPRINT_LINES};

/// What the reader last saw at a path. `Lines` keeps one fingerprint per line
/// (terminator included), so a later diff is exact but removed text is gone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Baseline {
    Text(Arc<str>),
    Lines(Arc<[u64]>),
}

impl Baseline {
    /// The retained form of freshly read content: text up to
    /// `MAX_DELTA_FILE_BYTES`, fingerprints above it. Blocking CPU for large text.
    pub(crate) fn retain(text: Arc<str>) -> Option<Self> {
        if text.len() as u64 <= MAX_DELTA_FILE_BYTES {
            return Some(Self::Text(text));
        }
        let lines = split_lines(&text);
        fingerprints(&lines).map(Self::Lines)
    }

    /// Bytes charged against `CACHE_BYTES_PER_REPO`.
    pub(crate) fn bytes(&self) -> usize {
        match self {
            Self::Text(text) => text.len(),
            Self::Lines(prints) => prints.len().saturating_mul(std::mem::size_of::<u64>()),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        match self {
            Self::Text(text) => text.is_empty(),
            Self::Lines(prints) => prints.is_empty(),
        }
    }

    /// The retained text, when this baseline kept it.
    #[cfg(test)]
    pub(crate) fn text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Lines(_) => None,
        }
    }
}

/// Lines with their terminators, the same tokens `similar`'s line diff uses.
pub(crate) fn split_lines(text: &str) -> Vec<&str> {
    text.split_inclusive('\n').collect()
}

/// `None` past `MAX_FINGERPRINT_LINES`: such a file has no retainable baseline.
pub(crate) fn fingerprints(lines: &[&str]) -> Option<Arc<[u64]>> {
    if lines.len() > MAX_FINGERPRINT_LINES {
        return None;
    }
    Some(lines.iter().map(|line| fingerprint(line)).collect())
}

/// SipHash with the fixed default keys: stable within the process, which is
/// the only lifetime a fingerprint has (baselines are never persisted).
pub(crate) fn fingerprint(line: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hasher.write(line.as_bytes());
    hasher.finish()
}
