use serde::{Deserialize, Serialize};

use super::Span;

/// A `consensus <name> { ... }` block for multi-agent verification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsensusDef {
    pub name: String,
    /// List of agent names participating.
    pub agents: Vec<String>,
    /// Consensus strategy (majority, unanimous, etc.).
    pub strategy: ConsensusStrategy,
    /// Required agreement threshold (e.g. 2 of 3).
    pub require: Option<ConsensusRequirement>,
    pub span: Span,
}

/// Strategy for reaching consensus.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsensusStrategy {
    Majority,
    Unanimous,
    Custom(String),
}

/// Agreement requirement: `N of M agree`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsensusRequirement {
    pub required: u32,
    pub total: u32,
}
