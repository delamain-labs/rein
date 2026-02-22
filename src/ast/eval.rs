use serde::{Deserialize, Serialize};

use super::{CompareOp, Span};

/// Action to take when an eval assertion fails.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EvalFailureAction {
    /// Block a named action (e.g. `block deploy`).
    Block { target: String },
    /// Escalate to a human.
    Escalate,
}

/// A single assertion in an eval block (e.g. `assert accuracy > 90%`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalAssertion {
    pub metric: String,
    pub op: CompareOp,
    pub value: String,
    pub span: Span,
}

/// An `eval { dataset: ..., assert ..., on failure: ... }` block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalDef {
    pub name: Option<String>,
    pub dataset: String,
    pub assertions: Vec<EvalAssertion>,
    pub on_failure: Option<EvalFailureAction>,
    pub span: Span,
}
