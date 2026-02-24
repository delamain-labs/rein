pub mod alerting;
pub mod approval;
pub mod audit;
pub mod budget;
pub mod channel;
pub mod circuit_breaker;
pub mod consensus;
pub mod engine;
pub mod eval;
pub mod events;
pub mod execution;
pub mod executor;
pub mod fleet;
pub mod guardrails;
pub mod injection;
pub mod interceptor;
pub mod memory;
pub mod observability;
pub mod observe;
pub mod otel_export;
pub mod permissions;
pub mod policy;
pub mod provider;
pub mod registry;
pub mod sandbox;
pub mod scenario;
pub mod schedule;
pub mod secrets;
pub mod webhook;
pub mod workflow;

use serde::{Deserialize, Serialize};

/// A tool invocation requested by the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub namespace: String,
    pub action: String,
    pub arguments: serde_json::Value,
}

/// The outcome of executing a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
}

/// A discrete event that occurs during an agent run.
///
/// # Stability
/// This enum is `#[non_exhaustive]`: new variants may be added in minor
/// versions without a semver major bump. Downstream crates that `match` on
/// `RunEvent` must include a wildcard (`_ => {}`) arm.
///
/// # Downstream usage
/// ```rust,ignore
/// // In a crate that depends on `rein`:
/// use rein::runtime::RunEvent;
///
/// fn handle(event: RunEvent) {
///     match event {
///         RunEvent::LlmCall { cost_cents, .. } => { /* ... */ }
///         RunEvent::StepFailed { step, error_kind, .. } => { /* ... */ }
///         _ => {} // required — new variants may be added in minor versions
///     }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunEvent {
    LlmCall {
        model: String,
        input_tokens: u64,
        output_tokens: u64,
        cost_cents: u64,
    },
    ToolCallAttempt {
        tool: ToolCall,
        allowed: bool,
        reason: Option<String>,
    },
    ToolCallResult {
        tool: ToolCall,
        result: ToolResult,
    },
    BudgetUpdate {
        spent_cents: u64,
        limit_cents: u64,
    },
    GuardrailTriggered {
        rule: String,
        action: String,
        blocked: bool,
    },
    CircuitBreakerTripped {
        name: String,
        failures: u32,
        threshold: u32,
    },
    PolicyPromotion {
        from_tier: String,
        to_tier: String,
    },
    PolicyDemotion {
        from_tier: String,
        to_tier: String,
        reason: String,
    },
    ApprovalRequested {
        step: String,
        channel: String,
        status: String,
    },
    EvalResult {
        metric: String,
        passed: bool,
        detail: String,
    },
    RunComplete {
        total_cost_cents: u64,
        total_tokens: u64,
    },
    /// A step's fallback was triggered after the primary step failed.
    StepFallback {
        step: String,
        fallback_step: String,
    },
    /// One iteration of a `for each` step.
    ForEachIteration {
        step: String,
        index: usize,
        total: usize,
    },
    /// A workflow's `auto resolve` conditions were met; remaining steps skipped.
    AutoResolved {
        step: String,
        condition: String,
    },
    /// A workflow step is about to begin execution.
    ///
    /// `index` is the **topological execution order** (0-based), which may differ
    /// from the step's declaration order in the `.rein` file when `depends_on` is
    /// used. Skipped steps do not emit `StepStarted`, so `index` values in a trace
    /// may not be contiguous.
    StepStarted {
        step: String,
        index: usize,
    },
    /// A workflow step completed successfully.
    StepCompleted {
        step: String,
    },
    /// Provider call exceeded the configured `stage_timeout_secs` on this turn.
    StageTimeout {
        turn: usize,
        timeout_secs: u64,
    },
    /// A step was skipped because one or more of its declared dependencies failed.
    StepSkipped {
        /// Name of the step that was skipped.
        step: String,
        /// Name of the immediate dependency that blocked this step. May itself be a
        /// skipped step rather than a directly failed step (transitive chains).
        /// Use the `StepFailed` event to find the root-cause step.
        /// Enables structured OTEL queries via `rein.step.blocked_dependency`.
        blocked_dependency: String,
        /// Human-readable reason (e.g. "dependency '`step_a`' failed").
        reason: String,
    },
    /// A workflow step failed during execution (soft failure — workflow continues).
    StepFailed {
        /// Name of the step that failed.
        step: String,
        /// Human-readable error description.
        reason: String,
        /// Machine-parseable error category in `snake_case` (e.g. `"agent_not_found"`,
        /// `"stage_failed"`). Mapped from the underlying `WorkflowError` variant at
        /// the emit site. OTEL dashboards and alerting rules should use this field
        /// rather than parsing `reason` with regex.
        ///
        /// Defaults to `"unknown"` when deserializing JSON produced before this field
        /// was added, so consumers can replay persisted event streams without errors.
        #[serde(default = "default_error_kind")]
        error_kind: String,
    },
}

/// Default value for `StepFailed::error_kind` when deserializing JSON that predates the field.
///
/// Returning `"unknown"` rather than `""` makes it detectable by consumers
/// (an empty string is ambiguous — was it explicitly set to empty or was it missing?).
fn default_error_kind() -> String {
    "unknown".to_string()
}

/// An ordered log of all events that occurred during a run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunTrace {
    pub events: Vec<RunEvent>,
    /// Real wall-clock offsets (ms from run start) captured at event-push time.
    /// Empty for deserialized or test-constructed traces; `to_structured` falls
    /// back to a fake counter when this is empty.
    #[serde(skip)]
    timestamps_ms: Vec<u64>,
}

impl RunTrace {
    /// Construct a trace from events with no timing information.
    /// `to_structured` will fall back to a monotonic counter for offsets.
    #[must_use]
    pub fn from_events(events: Vec<RunEvent>) -> Self {
        Self {
            timestamps_ms: Vec::new(),
            events,
        }
    }

    /// Construct a trace from events paired with real wall-clock offsets (ms
    /// from run start). `timestamps_ms` must be the same length as `events`.
    ///
    /// Note: `timestamps_ms` carries `#[serde(skip)]` and is **not** round-tripped
    /// through JSON. Deserialized traces fall back to the monotonic counter in
    /// `to_structured`. This is intentional: timestamps are only needed at
    /// export time, not during deserialization.
    #[must_use]
    pub(crate) fn from_events_timed(events: Vec<RunEvent>, timestamps_ms: Vec<u64>) -> Self {
        // Hard assert: this is called exclusively from `AgentEngine::finish`
        // where `RunState::push()` keeps the two vecs in sync. A length
        // mismatch is always a bug in the call site, not a runtime condition —
        // fail fast in all build profiles rather than silently degrading to the
        // fake counter.
        assert_eq!(
            events.len(),
            timestamps_ms.len(),
            "from_events_timed: events and timestamps_ms must have the same length"
        );
        Self {
            events,
            timestamps_ms,
        }
    }

    /// Serialize to pretty-printed JSON.
    ///
    /// # Errors
    /// Returns a serialization error if the trace cannot be serialized.
    ///
    /// # Note
    /// `timestamps_ms` is not serialized (`#[serde(skip)]`). A trace deserialized
    /// from JSON and then passed to `to_structured` will use fallback `(i * 100)`
    /// offsets rather than real wall-clock values. OTEL export should be done
    /// at run-time from the in-memory trace, not from a serialized round-trip.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Convert to a structured trace with timestamps and stats.
    #[must_use]
    pub fn to_structured(
        &self,
        agent_name: &str,
        started_at: &str,
        completed_at: &str,
        duration_ms: u64,
    ) -> StructuredTrace {
        let mut total_tokens = 0u64;
        let mut total_cost = 0u64;
        let mut llm_calls = 0u64;
        let mut tool_calls = 0u64;
        let mut tool_denied = 0u64;
        let has_real_timestamps = self.timestamps_ms.len() == self.events.len();

        let events: Vec<TimestampedEvent> = self
            .events
            .iter()
            .enumerate()
            .map(|(i, e)| {
                match e {
                    RunEvent::LlmCall {
                        input_tokens,
                        output_tokens,
                        cost_cents,
                        ..
                    } => {
                        llm_calls += 1;
                        total_tokens += input_tokens + output_tokens;
                        total_cost += cost_cents;
                    }
                    RunEvent::ToolCallAttempt { allowed, .. } => {
                        if *allowed {
                            tool_calls += 1;
                        } else {
                            tool_denied += 1;
                        }
                    }
                    _ => {}
                }
                let offset_ms = if has_real_timestamps {
                    self.timestamps_ms[i]
                } else {
                    (i as u64) * 100
                };
                TimestampedEvent {
                    offset_ms,
                    event: e.clone(),
                }
            })
            .collect();

        StructuredTrace {
            version: "1.0".to_string(),
            started_at: started_at.to_string(),
            completed_at: completed_at.to_string(),
            agent: agent_name.to_string(),
            events,
            stats: TraceStats {
                total_tokens,
                total_cost_cents: total_cost,
                llm_calls,
                tool_calls,
                tool_calls_denied: tool_denied,
                duration_ms,
            },
            is_partial: false,
        }
    }

    /// Write the structured trace to a file as JSON.
    ///
    /// `started_at` and `completed_at` must be RFC 3339 strings (e.g. from
    /// `chrono::Utc::now().to_rfc3339()`). `duration_ms` is the wall-clock
    /// run duration in milliseconds. All three values are recorded in the
    /// trace and used by OTLP exporters to produce accurate span timestamps.
    ///
    /// # Note
    /// Real per-event timestamps (`timestamps_ms`) are not written to the JSON
    /// file (`#[serde(skip)]`). If the file is later deserialized and passed to
    /// an OTEL exporter, fallback `(i * 100)` offsets are used instead of real
    /// wall-clock values. OTEL export should be performed from the in-memory
    /// trace at run-time, not from a deserialized JSON round-trip.
    ///
    /// # Errors
    /// Returns IO or serialization errors.
    pub fn write_to_file(
        &self,
        path: &std::path::Path,
        agent_name: &str,
        started_at: &str,
        completed_at: &str,
        duration_ms: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let trace = self.to_structured(agent_name, started_at, completed_at, duration_ms);
        let json = serde_json::to_string_pretty(&trace)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Produce a compact human-readable summary.
    #[must_use]
    pub fn summary(&self) -> String {
        Self::summarize_events(&self.events)
    }

    /// Format a slice of events into a human-readable summary string.
    /// Use this when you have a `Vec<RunEvent>` but do not need a full `RunTrace`.
    pub fn summarize_events(events: &[RunEvent]) -> String {
        let mut lines = Vec::new();
        let mut turn = 0_usize;
        for event in events {
            summarize_event(event, &mut lines, &mut turn);
        }
        lines.join("\n")
    }
}

/// Format a single `RunEvent` into a human-readable line and push it to `lines`.
/// `turn` tracks the current LLM call count for display.
#[allow(clippy::too_many_lines)]
fn summarize_event(event: &RunEvent, lines: &mut Vec<String>, turn: &mut usize) {
    match event {
        RunEvent::LlmCall {
            model,
            input_tokens,
            output_tokens,
            cost_cents,
        } => {
            *turn += 1;
            lines.push(format!(
                "[turn {turn}] LLM call ({model}): {input_tokens} in / {output_tokens} out, {cost_cents}¢"
            ));
        }
        RunEvent::ToolCallAttempt {
            tool,
            allowed,
            reason,
        } => {
            let status = if *allowed { "✓" } else { "✗" };
            let suffix = reason.as_ref().map_or(String::new(), |r| format!(" ({r})"));
            lines.push(format!(
                "  {status} tool: {}.{}{suffix}",
                tool.namespace, tool.action
            ));
        }
        RunEvent::ToolCallResult { tool, result } => {
            let status = if result.success { "ok" } else { "err" };
            let preview: String = result.output.chars().take(80).collect();
            lines.push(format!(
                "  → {}.{} [{status}]: {preview}",
                tool.namespace, tool.action
            ));
        }
        RunEvent::BudgetUpdate {
            spent_cents,
            limit_cents,
        } => {
            lines.push(format!("  budget: {spent_cents}¢ / {limit_cents}¢"));
        }
        RunEvent::GuardrailTriggered {
            rule,
            action,
            blocked,
        } => {
            let status = if *blocked { "BLOCKED" } else { "triggered" };
            lines.push(format!("  ⚠ guardrail [{status}]: {rule} ({action})"));
        }
        RunEvent::CircuitBreakerTripped {
            name,
            failures,
            threshold,
        } => {
            lines.push(format!(
                "  🔌 circuit breaker '{name}' tripped ({failures}/{threshold} failures)"
            ));
        }
        RunEvent::PolicyPromotion { from_tier, to_tier } => {
            lines.push(format!("  ⬆ policy: promoted {from_tier} → {to_tier}"));
        }
        RunEvent::PolicyDemotion {
            from_tier,
            to_tier,
            reason,
        } => {
            lines.push(format!(
                "  ⬇ policy: demoted {from_tier} → {to_tier} ({reason})"
            ));
        }
        RunEvent::ApprovalRequested {
            step,
            channel,
            status,
        } => {
            lines.push(format!(
                "  🛑 approval: step '{step}' via {channel}: {status}"
            ));
        }
        RunEvent::EvalResult {
            metric,
            passed,
            detail,
        } => {
            let status = if *passed { "✓" } else { "✗" };
            lines.push(format!("  {status} eval: {metric}: {detail}"));
        }
        RunEvent::RunComplete {
            total_cost_cents,
            total_tokens,
        } => {
            lines.push(format!(
                "Done. {total_tokens} tokens, {total_cost_cents}¢ total."
            ));
        }
        RunEvent::StepFallback {
            step,
            fallback_step,
        } => {
            lines.push(format!("  ↩ fallback: step '{step}' → '{fallback_step}'"));
        }
        RunEvent::ForEachIteration { step, index, total } => {
            lines.push(format!(
                "  ↻ for each: step '{step}' iteration {}/{total}",
                index + 1
            ));
        }
        RunEvent::AutoResolved { step, condition } => {
            lines.push(format!(
                "  ✓ auto resolved after step '{step}': {condition}"
            ));
        }
        RunEvent::StepStarted { step, index } => {
            lines.push(format!("  → step[{index}] '{step}' started"));
        }
        RunEvent::StepCompleted { step } => {
            lines.push(format!("  ✓ step '{step}' completed"));
        }
        RunEvent::StepFailed { step, reason, .. } => {
            lines.push(format!("  ✗ step '{step}' failed: {reason}"));
        }
        RunEvent::StageTimeout { turn, timeout_secs } => {
            // StageTimeout.turn is 0-indexed in the event struct (raw loop counter).
            // Add 1 for the human-readable display so "first LLM call timed out"
            // shows as "turn 1" rather than "turn 0", consistent with all other
            // 1-indexed turn references in human-facing summarize_event output. (#423)
            let display_turn = turn + 1;
            lines.push(format!(
                "  ✗ turn {display_turn} timed out after {timeout_secs}s"
            ));
        }
        RunEvent::StepSkipped { step, reason, .. } => {
            lines.push(format!("  ⏭ step '{step}' skipped: {reason}"));
        }
    }
}

/// A structured trace with metadata for serialization to file or stdout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredTrace {
    /// Schema version for forward compatibility.
    pub version: String,
    /// ISO 8601 timestamp when the run started.
    pub started_at: String,
    /// ISO 8601 timestamp when the run completed.
    pub completed_at: String,
    /// Agent name.
    pub agent: String,
    /// The events that occurred during the run.
    pub events: Vec<TimestampedEvent>,
    /// Aggregate statistics.
    pub stats: TraceStats,
    /// `true` when this trace was exported from a partial/timed-out run.
    /// OTEL consumers can filter on `rein.run.partial = "true"` to distinguish
    /// incomplete runs from normally-empty completions.
    #[serde(default)]
    pub is_partial: bool,
}

/// An event with a timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampedEvent {
    /// Monotonic offset in milliseconds from run start.
    pub offset_ms: u64,
    /// The event payload.
    #[serde(flatten)]
    pub event: RunEvent,
}

/// Aggregate statistics for a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStats {
    pub total_tokens: u64,
    pub total_cost_cents: u64,
    pub llm_calls: u64,
    pub tool_calls: u64,
    pub tool_calls_denied: u64,
    pub duration_ms: u64,
}

/// Errors that can occur during an agent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunError {
    /// Budget exhausted. Contains events up to and including the final
    /// `BudgetUpdate` event so callers can inspect exact spent/limit values.
    ///
    /// `partial_trace` carries `#[serde(skip)]` — in-process only; wire
    /// consumers see `{"budget_exceeded": {}}`.
    ///
    /// # BREAKING CHANGE
    /// Prior to adding `partial_trace`, this variant was a unit variant and
    /// serialized as the bare string `"budget_exceeded"`. It now serializes as
    /// `{"budget_exceeded": {}}` (an object with an empty body) because serde
    /// treats struct variants differently from unit variants even when all
    /// fields are skipped. Consumers that pattern-match on the raw JSON shape
    /// must update their deserialization logic.
    #[non_exhaustive]
    BudgetExceeded {
        #[serde(skip)]
        partial_trace: RunTrace,
    },
    PermissionDenied,
    ProviderError,
    ConfigError,
    /// Circuit breaker tripped. Contains events up to and including the
    /// `CircuitBreakerTripped` event so callers can inspect failures/threshold.
    ///
    /// `partial_trace` carries `#[serde(skip)]` — in-process only; wire
    /// consumers see `{"circuit_breaker_open": {}}`.
    ///
    /// # BREAKING CHANGE
    /// Prior to adding `partial_trace`, this variant was a unit variant and
    /// serialized as the bare string `"circuit_breaker_open"`. It now
    /// serializes as `{"circuit_breaker_open": {}}` (an object with an empty
    /// body). Consumers that pattern-match on the raw JSON shape must update
    /// their deserialization logic.
    #[non_exhaustive]
    CircuitBreakerOpen {
        #[serde(skip)]
        partial_trace: RunTrace,
    },
    GuardrailBlocked,
    EvalFailed,
    /// Provider call exceeded `stage_timeout_secs`. Contains events emitted
    /// up to (and including) the `StageTimeout` event so callers can inspect
    /// the partial trace (e.g. in tests or structured error reporting).
    ///
    /// `partial_trace` carries `#[serde(skip)]` — it is in-process only and
    /// must not appear on the wire. Wire consumers see `{"timeout": {}}`.
    ///
    /// # BREAKING CHANGE
    /// Prior to adding `#[serde(skip)]` to `partial_trace`, this variant
    /// serialized its events on the wire as `{"timeout": {"events": [...]}}`.
    /// It now serializes as `{"timeout": {}}` (empty object). Consumers that
    /// deserialize the raw JSON shape must update their logic.
    #[non_exhaustive]
    Timeout {
        #[serde(skip)]
        partial_trace: RunTrace,
    },
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BudgetExceeded { .. } => write!(f, "budget exceeded"),
            Self::PermissionDenied => write!(f, "permission denied"),
            Self::ProviderError => write!(f, "provider error"),
            Self::ConfigError => write!(f, "configuration error"),
            Self::CircuitBreakerOpen { .. } => write!(f, "circuit breaker open"),
            Self::GuardrailBlocked => write!(f, "guardrail blocked"),
            Self::EvalFailed => write!(f, "eval failed"),
            Self::Timeout { .. } => write!(f, "provider timed out"),
        }
    }
}

impl std::error::Error for RunError {}

#[cfg(test)]
mod tests;
