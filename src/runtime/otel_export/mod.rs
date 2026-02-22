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

/// Convert a structured trace to OTLP-compatible JSON.
pub fn to_otlp(trace: &StructuredTrace) -> OtelResourceSpans {
    let trace_id = pseudo_id(
        trace
            .stats
            .total_tokens
            .wrapping_add(trace.stats.duration_ms),
        16,
    );
    let root_span_id = pseudo_id(trace.stats.total_cost_cents.wrapping_add(1), 8);

    let mut spans = Vec::new();

    // Root span for the entire run
    let root_span = OtelSpan {
        trace_id: trace_id.clone(),
        span_id: root_span_id.clone(),
        parent_span_id: None,
        name: format!("rein.run.{}", trace.agent),
        kind: 1,                 // INTERNAL
        start_time_unix_nano: 0, // Would use real timestamps in production
        end_time_unix_nano: trace.stats.duration_ms * 1_000_000,
        attributes: vec![
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
        ],
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
            start_time_unix_nano: te.offset_ms * 1_000_000,
            end_time_unix_nano: te.offset_ms * 1_000_000,
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
