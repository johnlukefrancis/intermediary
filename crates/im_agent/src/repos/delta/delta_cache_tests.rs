// Path: crates/im_agent/src/repos/delta/delta_cache_tests.rs
// Description: Baseline cache tests - byte budget, LRU eviction and the rename carry

use std::sync::Arc;

use super::baseline_cache::BaselineCache;

fn text(size: usize) -> Arc<str> {
    Arc::from("x".repeat(size).as_str())
}

#[test]
fn cache_evicts_lru_by_bytes() {
    let mut cache = BaselineCache::new(300);
    cache.insert("a".to_string(), text(100));
    cache.insert("b".to_string(), text(100));
    cache.insert("c".to_string(), text(100));
    assert_eq!(cache.len(), 3);
    assert_eq!(cache.bytes(), 300);

    assert!(cache.get("a").is_some(), "touching a makes b the oldest");
    cache.insert("d".to_string(), text(100));

    assert_eq!(cache.bytes(), 300, "the cache stays inside its budget");
    assert!(
        cache.get("b").is_none(),
        "the least recently used entry went"
    );
    assert!(cache.get("a").is_some());
    assert!(cache.get("c").is_some());
    assert!(cache.get("d").is_some());

    cache.rename("d", "e");
    assert!(cache.get("d").is_none());
    assert!(cache.get("e").is_some(), "a rename carries the baseline");

    assert_eq!(cache.remove("e").map(|entry| entry.len()), Some(100));
    assert_eq!(cache.bytes(), 200);
}
