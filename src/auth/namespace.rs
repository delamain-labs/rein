//! Namespace-based multi-tenancy.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_lookup() {
        let mut mgr = NamespaceManager::new();
        mgr.create(Namespace::new("acme", "Acme Corp"));
        assert!(mgr.get("acme").is_some());
        assert!(mgr.get("missing").is_none());
    }

    #[test]
    fn isolation() {
        let mut mgr = NamespaceManager::new();
        mgr.create(Namespace::new("a", "Team A"));
        mgr.create(Namespace::new("b", "Team B"));
        assert_eq!(mgr.list().len(), 2);
        mgr.delete("a");
        assert_eq!(mgr.list().len(), 1);
    }
}

/// A namespace (tenant).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Namespace {
    pub id: String,
    pub display_name: String,
    pub metadata: HashMap<String, String>,
}

impl Namespace {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            display_name: name.into(),
            metadata: HashMap::new(),
        }
    }
}

/// Manages namespaces.
#[derive(Debug, Default)]
pub struct NamespaceManager {
    namespaces: HashMap<String, Namespace>,
}

impl NamespaceManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create(&mut self, ns: Namespace) {
        self.namespaces.insert(ns.id.clone(), ns);
    }

    pub fn get(&self, id: &str) -> Option<&Namespace> {
        self.namespaces.get(id)
    }

    pub fn delete(&mut self, id: &str) -> bool {
        self.namespaces.remove(id).is_some()
    }

    pub fn list(&self) -> Vec<&Namespace> {
        self.namespaces.values().collect()
    }
}
