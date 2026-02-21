use serde::{Deserialize, Serialize};

use super::{Span, ValueExpr};

/// A `provider <name> { ... }` definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderDef {
    pub name: String,
    /// The default model for this provider (e.g. `claude-haiku`, `gpt-4o`).
    pub model: Option<ValueExpr>,
    /// API key or credential reference (e.g. `env("ANTHROPIC_KEY")`).
    pub key: Option<ValueExpr>,
    pub span: Span,
}

/// A tool block: `tool zendesk { endpoint: "https://..." }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    /// The endpoint URL for this tool.
    pub endpoint: Option<ValueExpr>,
    /// Optional provider type (e.g. `rest_api`, `mcp`).
    pub provider: Option<ValueExpr>,
    /// Optional API key or credential reference.
    pub key: Option<ValueExpr>,
    pub span: Span,
}
