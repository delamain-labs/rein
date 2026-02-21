use serde::{Deserialize, Serialize};

use super::Span;

/// A `circuit_breaker { ... }` block for failure protection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CircuitBreakerDef {
    pub name: String,
    /// Number of failures before opening: `open after: N failures in M min`.
    pub failure_threshold: u32,
    /// Window in minutes for counting failures.
    pub window_minutes: u32,
    /// Duration in minutes before transitioning to half-open.
    pub half_open_after_minutes: u32,
    pub span: Span,
}
