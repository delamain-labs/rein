use serde::{Deserialize, Serialize};

use super::Span;

/// A `channel <name> { ... }` block for pub/sub messaging between agents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChannelDef {
    pub name: String,
    /// The message type (e.g. `PriceChange[]`).
    pub message_type: Option<String>,
    /// Retention period as a string (e.g. "7 days").
    pub retention: Option<String>,
    pub span: Span,
}
