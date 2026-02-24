//! OpenTelemetry-compatible trace export.
//!
//! Converts Rein's `StructuredTrace` into OTLP-compatible JSON spans
//! that can be sent to any OpenTelemetry collector.

use serde::{Deserialize, Serialize};

use super::StructuredTrace;

#[cfg(test)]
mod tests;

/// An OTLP-compatible span.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtelSpan {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub kind: u8,
    pub start_time_unix_nano: u64,
    pub end_time_unix_nano: u64,
    pub attributes: Vec<OtelAttribute>,
    pub status: OtelStatus,
}

/// An OTLP attribute key-value pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtelAttribute {
    pub key: String,
    pub value: OtelValue,
}

/// An OTLP attribute value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtelValue {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub string_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub int_value: Option<i64>,
}

/// OTLP span status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtelStatus {
    pub code: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// OTLP resource spans wrapper (top-level export format).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OtelResourceSpans {
    pub resource: OtelResource,
    pub scope_spans: Vec<OtelScopeSpans>,
}

/// OTLP resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtelResource {
    pub attributes: Vec<OtelAttribute>,
}

/// OTLP scope spans.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtelScopeSpans {
    pub scope: OtelScope,
    pub spans: Vec<OtelSpan>,
}

/// OTLP instrumentation scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtelScope {
    pub name: String,
    pub version: String,
}

fn attr_str(key: &str, value: &str) -> OtelAttribute {
    OtelAttribute {
        key: key.to_string(),
        value: OtelValue {
            string_value: Some(value.to_string()),
            int_value: None,
        },
    }
}

fn attr_int(key: &str, value: i64) -> OtelAttribute {
    OtelAttribute {
        key: key.to_string(),
        value: OtelValue {
            string_value: None,
            int_value: Some(value),
        },
    }
}

fn pseudo_id(seed: u64, len: usize) -> String {
    use std::fmt::Write;
    let mut hash = seed;
    let mut out = String::with_capacity(len * 2);
    for _ in 0..len {
        hash = hash.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        let _ = write!(out, "{:02x}", (hash >> 56) as u8);
    }
    out
}

/// Parse an RFC 3339 timestamp string and return nanoseconds since Unix epoch,
/// or `None` if the string cannot be parsed or represents a pre-epoch time.
pub(crate) fn try_rfc3339_to_unix_nanos(ts: &str) -> Option<u64> {
    use chrono::DateTime;
    DateTime::parse_from_rfc3339(ts)
        .ok()
        .and_then(|dt| dt.timestamp_nanos_opt())
        .and_then(|n| u64::try_from(n).ok())
}

/// Parse an RFC 3339 timestamp string and return nanoseconds since Unix epoch.
///
/// Emits a warning to stderr and falls back to `0` if the string cannot be
/// parsed or represents a pre-epoch time. A fallback value of `0` (Unix epoch)
/// will appear clearly wrong in any OTLP viewer and is therefore detectable.
fn rfc3339_to_unix_nanos(ts: &str) -> u64 {
    try_rfc3339_to_unix_nanos(ts).unwrap_or_else(|| {
        eprintln!(
            "rein[otel]: warning: could not parse timestamp '{ts}' as RFC 3339; \
             falling back to Unix epoch 0 — spans will have incorrect timestamps"
        );
        0
    })
}

pub fn to_otlp(trace: &StructuredTrace) -> OtelResourceSpans {
    let trace_id = pseudo_id(
        trace
            .stats
            .total_tokens
            .wrapping_add(trace.stats.duration_ms),
        16,
    );
    let root_span_id = pseudo_id(trace.stats.total_cost_cents.wrapping_add(1), 8);

    let start_ns = rfc3339_to_unix_nanos(&trace.started_at);
    // Use completed_at for end_ns so the root span reflects the actual wall-clock
    // end time. Falls back to start_ns + duration_ms if completed_at is unparseable.
    // Uses try_ variant to correctly distinguish parse failure from a legitimately
    // epoch-zero completed_at (avoiding silent fallback for valid epoch timestamps).
    let end_ns = try_rfc3339_to_unix_nanos(&trace.completed_at).unwrap_or_else(|| {
        eprintln!(
            "rein[otel]: warning: could not parse completed_at '{}' as RFC 3339; \
             falling back to start + duration — end timestamp may be approximate",
            trace.completed_at
        );
        start_ns.saturating_add(trace.stats.duration_ms.saturating_mul(1_000_000))
    });

    let mut spans = Vec::new();

    // Root span for the entire run
    let root_span = OtelSpan {
        trace_id: trace_id.clone(),
        span_id: root_span_id.clone(),
        parent_span_id: None,
        name: format!("rein.run.{}", trace.agent),
        kind: 1, // INTERNAL
        start_time_unix_nano: start_ns,
        end_time_unix_nano: end_ns,
        attributes: {
            let mut attrs = vec![
                attr_str("rein.agent.name", &trace.agent),
                attr_int("rein.tokens.total", trace.stats.total_tokens.cast_signed()),
                attr_int(
                    "rein.cost.cents",
                    trace.stats.total_cost_cents.cast_signed(),
                ),
                attr_int("rein.llm.calls", trace.stats.llm_calls.cast_signed()),
                attr_int("rein.tool.calls", trace.stats.tool_calls.cast_signed()),
                attr_int(
                    "rein.tool.denied",
                    trace.stats.tool_calls_denied.cast_signed(),
                ),
            ];
            // #430: mark partial/timed-out exports so dashboards can filter them.
            if trace.is_partial {
                attrs.push(attr_str("rein.run.partial", "true"));
            }
            attrs
        },
        status: OtelStatus {
            code: 1, // OK
            message: None,
        },
    };
    spans.push(root_span);

    // Child spans for each event
    for (i, te) in trace.events.iter().enumerate() {
        let span_id = pseudo_id(i as u64 + 100, 8);
        let (name, attrs) = event_to_span_data(&te.event);

        spans.push(OtelSpan {
            trace_id: trace_id.clone(),
            span_id,
            parent_span_id: Some(root_span_id.clone()),
            name,
            kind: 1,
            // Child event spans are point-in-time records (start == end).
            // RunEvents have no intrinsic duration — they mark when the event
            // occurred, not how long it took. OTLP collectors render these as
            // instant markers rather than duration bars.
            start_time_unix_nano: start_ns.saturating_add(te.offset_ms.saturating_mul(1_000_000)),
            end_time_unix_nano: start_ns.saturating_add(te.offset_ms.saturating_mul(1_000_000)),
            attributes: attrs,
            status: OtelStatus {
                code: 1,
                message: None,
            },
        });
    }

    OtelResourceSpans {
        resource: OtelResource {
            attributes: vec![
                attr_str("service.name", "rein"),
                attr_str("service.version", env!("CARGO_PKG_VERSION")),
            ],
        },
        scope_spans: vec![OtelScopeSpans {
            scope: OtelScope {
                name: "rein".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            spans,
        }],
    }
}

#[allow(clippy::too_many_lines)]
fn event_to_span_data(event: &super::RunEvent) -> (String, Vec<OtelAttribute>) {
    use super::RunEvent;
    match event {
        RunEvent::LlmCall {
            model,
            input_tokens,
            output_tokens,
            cost_cents,
        } => (
            "rein.llm.call".to_string(),
            vec![
                attr_str("rein.llm.model", model),
                attr_int("rein.llm.input_tokens", (*input_tokens).cast_signed()),
                attr_int("rein.llm.output_tokens", (*output_tokens).cast_signed()),
                attr_int("rein.llm.cost_cents", (*cost_cents).cast_signed()),
            ],
        ),
        RunEvent::ToolCallAttempt {
            tool,
            allowed,
            reason,
        } => {
            let mut attrs = vec![
                attr_str(
                    "rein.tool.name",
                    &format!("{}.{}", tool.namespace, tool.action),
                ),
                attr_str("rein.tool.allowed", &allowed.to_string()),
            ];
            if let Some(r) = reason {
                attrs.push(attr_str("rein.tool.reason", r));
            }
            ("rein.tool.attempt".to_string(), attrs)
        }
        RunEvent::ToolCallResult { tool, result } => (
            "rein.tool.result".to_string(),
            vec![
                attr_str(
                    "rein.tool.name",
                    &format!("{}.{}", tool.namespace, tool.action),
                ),
                attr_str("rein.tool.success", &result.success.to_string()),
            ],
        ),
        RunEvent::BudgetUpdate {
            spent_cents,
            limit_cents,
        } => (
            "rein.budget.update".to_string(),
            vec![
                attr_int("rein.budget.spent_cents", (*spent_cents).cast_signed()),
                attr_int("rein.budget.limit_cents", (*limit_cents).cast_signed()),
            ],
        ),
        RunEvent::RunComplete {
            total_cost_cents,
            total_tokens,
        } => (
            "rein.run.complete".to_string(),
            vec![
                attr_int(
                    "rein.run.total_cost_cents",
                    (*total_cost_cents).cast_signed(),
                ),
                attr_int("rein.run.total_tokens", (*total_tokens).cast_signed()),
            ],
        ),
        RunEvent::GuardrailTriggered {
            rule,
            action,
            blocked,
        } => (
            "rein.guardrail.triggered".to_string(),
            vec![
                attr_str("rein.guardrail.rule", rule),
                attr_str("rein.guardrail.action", action),
                attr_str("rein.guardrail.blocked", &blocked.to_string()),
            ],
        ),
        RunEvent::CircuitBreakerTripped {
            name,
            failures,
            threshold,
        } => (
            "rein.circuit_breaker.tripped".to_string(),
            vec![
                attr_str("rein.circuit_breaker.name", name),
                attr_int("rein.circuit_breaker.failures", i64::from(*failures)),
                attr_int("rein.circuit_breaker.threshold", i64::from(*threshold)),
            ],
        ),
        RunEvent::PolicyPromotion { from_tier, to_tier } => (
            "rein.policy.promotion".to_string(),
            vec![
                attr_str("rein.policy.from_tier", from_tier),
                attr_str("rein.policy.to_tier", to_tier),
            ],
        ),
        RunEvent::PolicyDemotion {
            from_tier,
            to_tier,
            reason,
        } => (
            "rein.policy.demotion".to_string(),
            vec![
                attr_str("rein.policy.from_tier", from_tier),
                attr_str("rein.policy.to_tier", to_tier),
                attr_str("rein.policy.reason", reason),
            ],
        ),
        RunEvent::ApprovalRequested {
            step,
            channel,
            status,
        } => (
            "rein.approval.requested".to_string(),
            vec![
                attr_str("rein.approval.step", step),
                attr_str("rein.approval.channel", channel),
                attr_str("rein.approval.status", status),
            ],
        ),
        RunEvent::EvalResult {
            metric,
            passed,
            detail,
        } => (
            "rein.eval.result".to_string(),
            vec![
                attr_str("rein.eval.metric", metric),
                attr_str("rein.eval.passed", &passed.to_string()),
                attr_str("rein.eval.detail", detail),
            ],
        ),
        RunEvent::StepFallback {
            step,
            fallback_step,
        } => (
            "rein.step.fallback".to_string(),
            vec![
                attr_str("rein.step.name", step),
                attr_str("rein.step.fallback", fallback_step),
            ],
        ),
        RunEvent::ForEachIteration { step, index, total } => (
            "rein.step.for_each".to_string(),
            vec![
                attr_str("rein.step.name", step),
                // -1 signals "index unknown / overflow" rather than i64::MAX
                // which would be indistinguishable from a legitimate large index.
                attr_int("rein.step.index", i64::try_from(*index).unwrap_or(-1)),
                attr_int("rein.step.total", i64::try_from(*total).unwrap_or(-1)),
            ],
        ),
        RunEvent::AutoResolved { step, condition } => (
            "rein.workflow.auto_resolved".to_string(),
            vec![
                attr_str("rein.step.name", step),
                attr_str("rein.auto_resolve.condition", condition),
            ],
        ),
        RunEvent::StepStarted { step, index } => (
            "rein.step.started".to_string(),
            vec![
                attr_str("rein.step.name", step),
                // -1 signals "index unknown / overflow" rather than i64::MAX,
                // which would be indistinguishable from a legitimate large index.
                // On 64-bit hosts usize→i64 conversion never actually overflows.
                attr_int("rein.step.index", i64::try_from(*index).unwrap_or(-1)),
            ],
        ),
        RunEvent::StepCompleted { step } => (
            "rein.step.completed".to_string(),
            vec![attr_str("rein.step.name", step)],
        ),
        RunEvent::StageTimeout { turn, timeout_secs } => (
            "rein.stage.timeout".to_string(),
            vec![
                // `rein.stage.turn` is intentionally 0-indexed — it stores the
                // raw loop-counter value for machine consumers (OTEL dashboards,
                // alerting rules). The human-readable summary adds +1 for display
                // (see `summarize_event`). Do not change this to 1-indexed: it
                // would break existing OTEL queries.
                // -1 signals overflow (same convention as rein.step.index): both
                // turn and timeout_secs are non-negative in domain, so -1 is
                // clearly out-of-range and distinguishable from a legitimate value.
                // On 64-bit hosts these conversions never actually overflow.
                attr_int("rein.stage.turn", i64::try_from(*turn).unwrap_or(-1)),
                attr_int(
                    "rein.stage.timeout_secs",
                    i64::try_from(*timeout_secs).unwrap_or(-1),
                ),
            ],
        ),
        RunEvent::StepSkipped {
            step,
            blocked_dependency,
            reason,
        } => (
            "rein.step.skipped".to_string(),
            vec![
                attr_str("rein.step.name", step),
                attr_str("rein.step.blocked_dependency", blocked_dependency),
                attr_str("rein.step.reason", reason),
            ],
        ),
        RunEvent::StepFailed {
            step,
            reason,
            error_kind,
        } => (
            "rein.step.failed".to_string(),
            vec![
                attr_str("rein.step.name", step),
                attr_str("rein.step.reason", reason),
                attr_str("rein.step.error_kind", error_kind),
            ],
        ),
    }
}

/// Serialize OTLP resource spans to JSON.
///
/// # Errors
/// Returns an error if serialization fails.
pub fn to_otlp_json(trace: &StructuredTrace) -> Result<String, serde_json::Error> {
    let resource_spans = to_otlp(trace);
    serde_json::to_string_pretty(&resource_spans)
}

/// Determines how OTEL traces are exported after a run.
#[derive(Debug, Clone, Default)]
pub enum OtelMode {
    /// No export (default).
    #[default]
    None,
    /// Write OTLP JSON to a timestamped file on completion.
    FileOnComplete,
    /// Print OTLP JSON spans to stdout on completion.
    /// `metrics` restricts which event types are included.
    StdoutOnComplete {
        /// Metric names to include ("latency", "cost", "`tool_calls`", "guardrails").
        /// An empty vec means include all events.
        metrics: Vec<String>,
    },
}
