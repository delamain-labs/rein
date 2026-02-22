use super::*;

#[test]
fn working_memory_set_get() {
    let store = MemoryStore::new();
    store.set_working("key1", "value1");
    assert_eq!(store.get_working("key1"), Some("value1".to_string()));
}

#[test]
fn working_memory_missing_key() {
    let store = MemoryStore::new();
    assert_eq!(store.get_working("missing"), None);
}

#[test]
fn session_memory_set_get() {
    let store = MemoryStore::new();
    store.set_session("session_key", "session_value");
    assert_eq!(
        store.get_session("session_key"),
        Some("session_value".to_string())
    );
}

#[test]
fn get_checks_working_first() {
    let store = MemoryStore::new();
    store.set_working("key", "working_value");
    store.set_session("key", "session_value");
    assert_eq!(store.get("key"), Some("working_value".to_string()));
}

#[test]
fn get_falls_back_to_session() {
    let store = MemoryStore::new();
    store.set_session("key", "session_value");
    assert_eq!(store.get("key"), Some("session_value".to_string()));
}

#[test]
fn clear_working_removes_entries() {
    let store = MemoryStore::new();
    store.set_working("key1", "v1");
    store.set_working("key2", "v2");
    assert_eq!(store.working_len(), 2);

    store.clear_working();
    assert_eq!(store.working_len(), 0);
    assert_eq!(store.get_working("key1"), None);
}

#[test]
fn clear_working_does_not_affect_session() {
    let store = MemoryStore::new();
    store.set_working("wk", "working");
    store.set_session("sk", "session");

    store.clear_working();
    assert_eq!(store.get_working("wk"), None);
    assert_eq!(store.get_session("sk"), Some("session".to_string()));
}

#[test]
fn overwrite_working_value() {
    let store = MemoryStore::new();
    store.set_working("key", "v1");
    store.set_working("key", "v2");
    assert_eq!(store.get_working("key"), Some("v2".to_string()));
    assert_eq!(store.working_len(), 1);
}

#[test]
fn default_creates_empty_store() {
    let store = MemoryStore::default();
    assert_eq!(store.working_len(), 0);
    assert_eq!(store.session_len(), 0);
}
