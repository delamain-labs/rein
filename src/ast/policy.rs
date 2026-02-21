use serde::{Deserialize, Serialize};

use super::workflow::WhenExpr;
use super::Span;

/// A single tier within a policy block.
///
/// Example: `tier supervised { promote when accuracy > 95% }`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyTier {
    pub name: String,
    /// Condition that triggers promotion to the next tier.
    pub promote_when: Option<WhenExpr>,
    pub span: Span,
}

/// A `policy { ... }` block defining progressive trust tiers.
///
/// Example:
/// ```rein
/// policy {
///     tier supervised { promote when accuracy > 95% }
///     tier autonomous { promote when accuracy > 99% }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyDef {
    pub tiers: Vec<PolicyTier>,
    pub span: Span,
}
