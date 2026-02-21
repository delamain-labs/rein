use serde::{Deserialize, Serialize};

use super::Span;

/// Scaling configuration for a fleet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScalingConfig {
    pub min: u32,
    pub max: u32,
    pub span: Span,
}

/// A `fleet <name> { ... }` block for agent group management.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FleetDef {
    pub name: String,
    /// Agent names in this fleet.
    pub agents: Vec<String>,
    /// Policy name reference.
    pub policy: Option<String>,
    /// Daily budget in cents.
    pub budget: Option<u64>,
    /// Scaling configuration.
    pub scaling: Option<ScalingConfig>,
    pub span: Span,
}
