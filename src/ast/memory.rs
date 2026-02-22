use serde::{Deserialize, Serialize};

use super::Span;

/// A tier in the memory system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryTier {
    Working,
    Session,
    Knowledge,
}

/// Configuration for a single memory tier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryTierDef {
    pub tier: MemoryTier,
    /// Optional TTL (e.g. "30m", "24h").
    pub ttl: Option<String>,
    /// Optional max entries.
    pub max_entries: Option<u64>,
    /// Optional storage backend (e.g. "redis", "sqlite").
    pub backend: Option<String>,
    pub span: Span,
}

/// A `memory { working { ... } session { ... } knowledge { ... } }` block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryDef {
    pub name: Option<String>,
    pub tiers: Vec<MemoryTierDef>,
    pub span: Span,
}
