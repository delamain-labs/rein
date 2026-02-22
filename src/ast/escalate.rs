use serde::{Deserialize, Serialize};

use super::Span;

/// An `escalate to human via channel(target)` expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EscalateDef {
    /// Who to escalate to (e.g. "human", "manager").
    pub target: String,
    /// Channel and destination (e.g. "slack", "email").
    pub channel: String,
    /// Channel-specific destination (e.g. "#refunds", "team@co.com").
    pub destination: String,
    pub span: Span,
}
