//! Agent memory system with tiered storage.
//!
//! Three tiers: working (in-process), session (persisted per session),
//! and knowledge (long-term retrieval).

use std::collections::HashMap;
use std::sync::Mutex;

#[cfg(test)]
mod tests;

/// A memory entry with metadata.
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub key: String,
    pub value: String,
    pub tier: MemoryTier,
    pub created_at_ms: u64,
    pub ttl_ms: Option<u64>,
}

/// Memory tier matching the DSL's tier definitions.
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryTier {
    Working,
    Session,
    Knowledge,
}

/// In-process memory store for agent state.
///
/// Working memory lives in-process and is cleared per run.
/// Session memory persists across runs (would use file/db in production).
/// Knowledge memory is read-only retrieval (would use embeddings in production).
pub struct MemoryStore {
    working: Mutex<HashMap<String, String>>,
    session: Mutex<HashMap<String, String>>,
}

impl MemoryStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            working: Mutex::new(HashMap::new()),
            session: Mutex::new(HashMap::new()),
        }
    }

    /// Store a value in working memory.
    pub fn set_working(&self, key: impl Into<String>, value: impl Into<String>) {
        self.working
            .lock()
            .expect("working memory mutex poisoned")
            .insert(key.into(), value.into());
    }

    /// Retrieve from working memory.
    #[must_use]
    pub fn get_working(&self, key: &str) -> Option<String> {
        self.working
            .lock()
            .expect("working memory mutex poisoned")
            .get(key)
            .cloned()
    }

    /// Store a value in session memory (persists across turns).
    pub fn set_session(&self, key: impl Into<String>, value: impl Into<String>) {
        self.session
            .lock()
            .expect("session memory mutex poisoned")
            .insert(key.into(), value.into());
    }

    /// Retrieve from session memory.
    #[must_use]
    pub fn get_session(&self, key: &str) -> Option<String> {
        self.session
            .lock()
            .expect("session memory mutex poisoned")
            .get(key)
            .cloned()
    }

    /// Get from any tier (working first, then session).
    #[must_use]
    pub fn get(&self, key: &str) -> Option<String> {
        self.get_working(key).or_else(|| self.get_session(key))
    }

    /// Clear working memory (called between runs).
    pub fn clear_working(&self) {
        self.working
            .lock()
            .expect("working memory mutex poisoned")
            .clear();
    }

    /// Number of entries in working memory.
    #[must_use]
    pub fn working_len(&self) -> usize {
        self.working
            .lock()
            .expect("working memory mutex poisoned")
            .len()
    }

    /// Number of entries in session memory.
    #[must_use]
    pub fn session_len(&self) -> usize {
        self.session
            .lock()
            .expect("session memory mutex poisoned")
            .len()
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}
