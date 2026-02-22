use super::*;

#[test]
fn execution_id_unique() {
    let a = ExecutionId::generate("wf", "trigger");
    let b = ExecutionId::generate("wf", "trigger");
    // Different timestamps should produce different IDs
    assert_ne!(a, b);
    assert!(a.0.starts_with("exec-"));
}

#[test]
fn dedup_cache_miss_then_hit() {
    let mut cache = DeduplicationCache::new();
    let key = DeduplicationCache::compute_key("step1", "search", "query=hello");
    assert!(cache.get(&key).is_none());
    cache.insert(key.clone(), "result".to_string());
    assert_eq!(cache.get(&key), Some("result"));
}

#[test]
fn dedup_key_deterministic() {
    let a = DeduplicationCache::compute_key("s", "t", "a");
    let b = DeduplicationCache::compute_key("s", "t", "a");
    assert_eq!(a, b);
}

#[test]
fn dedup_key_different_inputs() {
    let a = DeduplicationCache::compute_key("s1", "t", "a");
    let b = DeduplicationCache::compute_key("s2", "t", "a");
    assert_ne!(a, b);
}

#[test]
fn retry_tracker_allows_up_to_max() {
    let mut tracker = RetryTracker::new(3);
    assert!(tracker.attempt("step1")); // 1
    assert!(tracker.attempt("step1")); // 2
    assert!(tracker.attempt("step1")); // 3
    assert!(!tracker.attempt("step1")); // 4 — exceeds
}

#[test]
fn retry_tracker_reset() {
    let mut tracker = RetryTracker::new(2);
    tracker.attempt("step1");
    tracker.attempt("step1");
    assert!(!tracker.attempt("step1"));
    tracker.reset("step1");
    assert!(tracker.attempt("step1")); // reset, starts fresh
}

#[test]
fn retry_tracker_independent_keys() {
    let mut tracker = RetryTracker::new(1);
    assert!(tracker.attempt("a"));
    assert!(!tracker.attempt("a"));
    assert!(tracker.attempt("b")); // different key
}
