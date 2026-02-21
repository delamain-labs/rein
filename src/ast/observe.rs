use serde::{Deserialize, Serialize};

use super::{Span, WhenExpr};

/// An `observe <name> { ... }` block for monitoring and metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObserveDef {
    pub name: String,
    /// What to trace (e.g. "all steps").
    pub trace: Option<String>,
    /// Metrics to collect.
    pub metrics: Vec<String>,
    /// Alert condition: `alert when { ... }`.
    pub alert_when: Option<WhenExpr>,
    /// Export target (e.g. "prometheus", "datadog").
    pub export: Option<String>,
    pub span: Span,
}
