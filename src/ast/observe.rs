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
    /// Export target. Supported at runtime: `"otlp"`, `"stdout"`.
    /// Other values (e.g. `"prometheus"`, `"datadog"`) parse but are not yet
    /// implemented — the strict-mode validator emits `W_EXPORT_UNSUPPORTED`.
    pub export: Option<String>,
    pub span: Span,
}
