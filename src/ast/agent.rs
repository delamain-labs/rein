use serde::{Deserialize, Serialize};

use super::{Span, ValueExpr};

/// A monetary cap constraint on a capability (`up to $<amount>`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Constraint {
    MonetaryCap { amount: u64, currency: String },
}

/// A single tool capability, e.g. `zendesk.read_ticket` or `zendesk.refund up to $50`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Capability {
    pub namespace: String,
    pub action: String,
    pub constraint: Option<Constraint>,
    pub span: Span,
}

/// A spending budget, e.g. `budget: $0.03 per ticket`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Budget {
    pub amount: u64,
    pub currency: String,
    pub unit: String,
    pub span: Span,
}

/// A single `agent <name> { ... }` definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentDef {
    pub name: String,
    pub model: Option<ValueExpr>,
    pub can: Vec<Capability>,
    pub cannot: Vec<Capability>,
    pub budget: Option<Budget>,
    pub guardrails: Option<GuardrailsDef>,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Defaults types
// ---------------------------------------------------------------------------

/// A `defaults { model: ..., budget: ... }` block providing project-level defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefaultsDef {
    pub model: Option<ValueExpr>,
    pub budget: Option<Budget>,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Guardrails types
// ---------------------------------------------------------------------------

/// A `guardrails { ... }` block containing named sections.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuardrailsDef {
    pub sections: Vec<GuardrailSection>,
    pub span: Span,
}

/// A named section within guardrails, e.g. `output_filter { pii_detection: redact }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuardrailSection {
    pub name: String,
    pub rules: Vec<GuardrailRule>,
    pub span: Span,
}

/// A key-value rule within a guardrail section, e.g. `pii_detection: redact`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuardrailRule {
    pub key: String,
    pub value: String,
    pub span: Span,
}
