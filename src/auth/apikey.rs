//! API key management.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_validate() {
        let mut mgr = ApiKeyManager::new();
        let key = mgr.create("test-key", Some("acme"));
        assert!(mgr.validate(&key.key).is_some());
    }

    #[test]
    fn revoke_key() {
        let mut mgr = ApiKeyManager::new();
        let key = mgr.create("k", None);
        assert!(mgr.validate(&key.key).is_some());
        mgr.revoke(&key.key_hash);
        assert!(mgr.validate(&key.key).is_none());
    }

    #[test]
    fn list_keys() {
        let mut mgr = ApiKeyManager::new();
        mgr.create("a", Some("ns1"));
        mgr.create("b", Some("ns1"));
        mgr.create("c", Some("ns2"));
        assert_eq!(mgr.list_for_namespace("ns1").len(), 2);
    }

    #[test]
    fn invalid_key_rejected() {
        let mgr = ApiKeyManager::new();
        assert!(mgr.validate("rein_invalid_key").is_none());
    }
}

/// An API key entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub name: String,
    /// The plaintext key (only available at creation time).
    pub key: String,
    /// SHA-256 hash of the key (stored for validation).
    pub key_hash: String,
    /// Associated namespace (tenant).
    pub namespace: Option<String>,
    pub active: bool,
}

/// API key manager.
#[derive(Debug, Default)]
pub struct ApiKeyManager {
    /// Keys indexed by hash.
    keys: HashMap<String, ApiKey>,
}

impl ApiKeyManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new API key.
    pub fn create(&mut self, name: &str, namespace: Option<&str>) -> ApiKey {
        let key = generate_key();
        let key_hash = hash_key(&key);
        let entry = ApiKey {
            name: name.to_string(),
            key: key.clone(),
            key_hash: key_hash.clone(),
            namespace: namespace.map(String::from),
            active: true,
        };
        self.keys.insert(key_hash, entry.clone());
        entry
    }

    /// Validate a key and return the entry if valid.
    pub fn validate(&self, key: &str) -> Option<&ApiKey> {
        let hash = hash_key(key);
        self.keys.get(&hash).filter(|k| k.active)
    }

    /// Revoke a key by hash.
    pub fn revoke(&mut self, key_hash: &str) {
        if let Some(key) = self.keys.get_mut(key_hash) {
            key.active = false;
        }
    }

    /// List keys for a namespace.
    pub fn list_for_namespace(&self, namespace: &str) -> Vec<&ApiKey> {
        self.keys
            .values()
            .filter(|k| k.namespace.as_deref() == Some(namespace) && k.active)
            .collect()
    }
}

fn generate_key() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut hasher = Sha256::new();
    hasher.update(format!("rein-key-{nanos}"));
    format!("rein_{:x}", hasher.finalize())
        .chars()
        .take(40)
        .collect()
}

fn hash_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key);
    format!("{:x}", hasher.finalize())
}
