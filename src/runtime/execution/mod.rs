//! Durable execution with idempotent retries and deduplication.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[cfg(test)]
mod tests;

/// Unique execution identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExecutionId(pub String);

impl ExecutionId {
    /// Generate an execution ID from workflow + trigger + timestamp.
    pub fn generate(workflow: &str, trigger: &str) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut hasher = Sha256::new();
        hasher.update(format!("{workflow}:{trigger}:{nanos}"));
        Self(format!("exec-{:x}", hasher.finalize()).chars().take(24).collect())
    }
}

/// Tool call deduplication cache.
#[derive(Debug, Default)]
pub struct DeduplicationCache {
    /// Maps `(step_name, tool_call_hash)` to a cached result.
    cache: HashMap<String, String>,
}

impl DeduplicationCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Compute a dedup key for a tool call.
    pub fn compute_key(step: &str, tool: &str, args: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("{step}:{tool}:{args}"));
        format!("{:x}", hasher.finalize()).chars().take(16).collect()
    }

    /// Check if a result is cached. Returns the cached result if found.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.cache.get(key).map(String::as_str)
    }

    /// Store a result in the cache.
    pub fn insert(&mut self, key: String, result: String) {
        self.cache.insert(key, result);
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

/// Idempotent retry tracker.
#[derive(Debug, Default)]
pub struct RetryTracker {
    attempts: HashMap<String, u32>,
    max_retries: u32,
}

impl RetryTracker {
    pub fn new(max_retries: u32) -> Self {
        Self {
            attempts: HashMap::new(),
            max_retries,
        }
    }

    /// Record an attempt. Returns `false` if max retries exceeded.
    pub fn attempt(&mut self, key: &str) -> bool {
        let count = self.attempts.entry(key.to_string()).or_insert(0);
        *count += 1;
        *count <= self.max_retries
    }

    /// Get the number of attempts for a key.
    pub fn attempts(&self, key: &str) -> u32 {
        self.attempts.get(key).copied().unwrap_or(0)
    }

    /// Reset attempts for a key (after successful retry).
    pub fn reset(&mut self, key: &str) {
        self.attempts.remove(key);
    }
}
