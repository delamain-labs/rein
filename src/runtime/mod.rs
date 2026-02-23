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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

/// An ordered log of all events that occurred during a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunTrace {
    pub events: Vec<RunEvent>,
}

impl RunTrace {
    /// Serialize to pretty-printed JSON.
    ///
    /// # Errors
    /// Returns a serialization error if the trace cannot be serialized.
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
                TimestampedEvent {
                    offset_ms: (i as u64) * 100,
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
        }
    }

    /// Write the structured trace to a file as JSON.
    ///
    /// `started_at` and `completed_at` must be RFC 3339 strings (e.g. from
    /// `chrono::Utc::now().to_rfc3339()`). `duration_ms` is the wall-clock
    /// run duration in milliseconds. All three values are recorded in the
    /// trace and used by OTLP exporters to produce accurate span timestamps.
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
    BudgetExceeded,
    PermissionDenied,
    ProviderError,
    ConfigError,
    CircuitBreakerOpen,
    GuardrailBlocked,
    EvalFailed,
}

#[cfg(test)]
mod tests;
