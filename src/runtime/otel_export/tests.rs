use super::*;
use crate::runtime::{RunEvent, RunTrace, ToolCall, ToolResult};

fn sample_trace() -> StructuredTrace {
    let events = vec![
        RunEvent::LlmCall {
            model: "gpt-4".to_string(),
            input_tokens: 100,
            output_tokens: 50,
            cost_cents: 5,
        },
        RunEvent::ToolCallAttempt {
            tool: ToolCall {
                namespace: "files".to_string(),
                action: "read".to_string(),
                arguments: serde_json::json!({}),
            },
            allowed: true,
            reason: None,
        },
        RunEvent::ToolCallResult {
            tool: ToolCall {
                namespace: "files".to_string(),
                action: "read".to_string(),
                arguments: serde_json::json!({}),
            },
            result: ToolResult {
                success: true,
                output: "file contents".to_string(),
            },
        },
        RunEvent::BudgetUpdate {
            spent_cents: 5,
            limit_cents: 100,
        },
        RunEvent::RunComplete {
            total_cost_cents: 5,
            total_tokens: 150,
        },
    ];

    let trace = RunTrace::from_events(events);
    trace.to_structured(
        "test_agent",
        "2026-01-01T00:00:00Z",
        "2026-01-01T00:00:01Z",
        1000,
    )
}

#[test]
fn otlp_export_produces_valid_structure() {
    let trace = sample_trace();
    let resource_spans = to_otlp(&trace);

    assert_eq!(resource_spans.scope_spans.len(), 1);
    let spans = &resource_spans.scope_spans[0].spans;

    // Root span + 5 event spans
    assert_eq!(spans.len(), 6);

    // Root span
    assert!(spans[0].name.starts_with("rein.run."));
    assert!(spans[0].parent_span_id.is_none());

    // All child spans have parent
    for span in &spans[1..] {
        assert!(span.parent_span_id.is_some());
        assert_eq!(span.parent_span_id.as_ref().unwrap(), &spans[0].span_id);
    }
}

#[test]
fn otlp_export_has_correct_span_names() {
    let trace = sample_trace();
    let resource_spans = to_otlp(&trace);
    let spans = &resource_spans.scope_spans[0].spans;

    assert_eq!(spans[0].name, "rein.run.test_agent");
    assert_eq!(spans[1].name, "rein.llm.call");
    assert_eq!(spans[2].name, "rein.tool.attempt");
    assert_eq!(spans[3].name, "rein.tool.result");
    assert_eq!(spans[4].name, "rein.budget.update");
    assert_eq!(spans[5].name, "rein.run.complete");
}

#[test]
fn otlp_json_serializes() {
    let trace = sample_trace();
    let json = to_otlp_json(&trace).expect("should serialize");
    assert!(json.contains("rein.run.test_agent"));
    assert!(json.contains("rein.agent.name"));
}

#[test]
fn otlp_resource_has_service_info() {
    let trace = sample_trace();
    let resource_spans = to_otlp(&trace);
    let attrs = &resource_spans.resource.attributes;

    let service_name = attrs.iter().find(|a| a.key == "service.name").unwrap();
    assert_eq!(service_name.value.string_value.as_deref(), Some("rein"));
}

// #329: root span must have real timestamps derived from started_at/completed_at,
// not hardcoded zeros.
// sample_trace: started_at = "2026-01-01T00:00:00Z", completed_at = "2026-01-01T00:00:01Z"
#[test]
fn otlp_root_span_has_real_timestamps() {
    let trace = sample_trace();
    let root = &to_otlp(&trace).scope_spans[0].spans[0];

    // 2026-01-01T00:00:00Z = 1767225600 seconds since Unix epoch
    let expected_start_ns: u64 = 1_767_225_600 * 1_000_000_000;
    assert_eq!(
        root.start_time_unix_nano, expected_start_ns,
        "root span start must equal started_at converted to nanoseconds"
    );
    // 2026-01-01T00:00:01Z = 1767225601 seconds since Unix epoch (completed_at)
    let expected_end_ns: u64 = 1_767_225_601 * 1_000_000_000;
    assert_eq!(
        root.end_time_unix_nano, expected_end_ns,
        "root span end must equal completed_at converted to nanoseconds"
    );
}

// #343: unparseable started_at falls back to epoch 0 (detectable sentinel in OTLP viewers).
#[test]
fn invalid_started_at_falls_back_to_epoch_zero() {
    let trace =
        RunTrace::from_events(vec![]).to_structured("agent", "not-a-date", "not-a-date", 500);
    let root = &to_otlp(&trace).scope_spans[0].spans[0];
    assert_eq!(
        root.start_time_unix_nano, 0,
        "unparseable started_at must produce sentinel 0"
    );
}

// #343: pre-epoch started_at (negative i64) falls back to epoch 0.
#[test]
fn pre_epoch_started_at_falls_back_to_epoch_zero() {
    let trace = RunTrace::from_events(vec![]).to_structured("agent", "1960-06-15T12:00:00Z", "", 0);
    let root = &to_otlp(&trace).scope_spans[0].spans[0];
    assert_eq!(
        root.start_time_unix_nano, 0,
        "pre-epoch started_at must produce sentinel 0"
    );
}

// #344: unparseable completed_at falls back to start_ns + duration instead of staying silent.
// This is tested indirectly: the fallback produces a non-zero end_ns equal to start + duration.
#[test]
fn invalid_completed_at_falls_back_to_start_plus_duration() {
    let started_at = "2026-01-01T00:00:00Z";
    // Derive start_ns from the same helper to keep the test self-consistent.
    let start_ns: u64 =
        try_rfc3339_to_unix_nanos(started_at).expect("test timestamp must be valid RFC 3339");
    let duration_ms: u64 = 2_000;
    let trace =
        RunTrace::from_events(vec![]).to_structured("agent", started_at, "not-a-date", duration_ms);
    let root = &to_otlp(&trace).scope_spans[0].spans[0];
    assert_eq!(
        root.end_time_unix_nano,
        start_ns.saturating_add(duration_ms.saturating_mul(1_000_000)),
        "invalid completed_at must fall back to start_ns + duration_ms * 1_000_000"
    );
}

#[test]
fn otlp_root_span_has_stats() {
    let trace = sample_trace();
    let resource_spans = to_otlp(&trace);
    let root = &resource_spans.scope_spans[0].spans[0];

    let tokens = root
        .attributes
        .iter()
        .find(|a| a.key == "rein.tokens.total")
        .unwrap();
    assert_eq!(tokens.value.int_value, Some(150));

    let cost = root
        .attributes
        .iter()
        .find(|a| a.key == "rein.cost.cents")
        .unwrap();
    assert_eq!(cost.value.int_value, Some(5));
}

// #425: rein.stage.turn in StageTimeout span must emit the raw turn value as an
// integer attribute. The overflow sentinel is -1 (same convention as rein.step.index
// — clearly out-of-domain since turns are non-negative).
#[test]
fn stage_timeout_turn_otel_attribute_is_integer_value() {
    use crate::runtime::RunEvent;
    use crate::runtime::RunTrace;

    let events = vec![RunEvent::StageTimeout {
        turn: 0,
        timeout_secs: 5,
    }];
    let trace = RunTrace::from_events(events);
    let structured = trace.to_structured(
        "test_agent",
        "2026-01-01T00:00:00Z",
        "2026-01-01T00:00:01Z",
        1000,
    );
    let resource_spans = to_otlp(&structured);
    let spans = &resource_spans.scope_spans[0].spans;

    let timeout_span = spans
        .iter()
        .find(|s| s.name == "rein.stage.timeout")
        .expect("must have a rein.stage.timeout span");

    let turn_attr = timeout_span
        .attributes
        .iter()
        .find(|a| a.key == "rein.stage.turn")
        .expect("must have rein.stage.turn attribute");

    // turn 0 converts to i64 without overflow → attribute value must be 0.
    assert_eq!(
        turn_attr.value.int_value,
        Some(0),
        "rein.stage.turn must equal the raw turn value; got: {:?}",
        turn_attr.value
    );
}

// #430: export_partial must mark the root span with rein.run.partial = "true"
// so dashboards can distinguish incomplete runs from normal empty completions.
#[test]
fn partial_trace_root_span_has_partial_attribute() {
    let mut trace = sample_trace();
    trace.is_partial = true;

    let resource_spans = to_otlp(&trace);
    let root = &resource_spans.scope_spans[0].spans[0];

    let partial_attr = root.attributes.iter().find(|a| a.key == "rein.run.partial");

    assert!(
        partial_attr.is_some(),
        "partial trace must have rein.run.partial attribute on root span; attributes: {:?}",
        root.attributes
    );
    assert_eq!(
        partial_attr.unwrap().value.string_value.as_deref(),
        Some("true"),
        "rein.run.partial must be \"true\""
    );
}

// --- #452: rein.step.error_kind OTEL attribute ---

/// #452: StepFailed events must emit `rein.step.error_kind` as a string attribute
/// so OTEL dashboards can filter by failure mode without regex on rein.step.reason.
#[test]
fn step_failed_otel_span_includes_error_kind_attribute() {
    use crate::runtime::RunEvent;
    use crate::runtime::RunTrace;

    let events = vec![RunEvent::StepFailed {
        step: "deploy".to_string(),
        reason: "agent not found: bot".to_string(),
        error_kind: crate::runtime::StepErrorKind::AgentNotFound,
    }];
    let trace = RunTrace::from_events(events);
    let structured = trace.to_structured(
        "test_agent",
        "2026-01-01T00:00:00Z",
        "2026-01-01T00:00:01Z",
        1000,
    );
    let resource_spans = to_otlp(&structured);
    let spans = &resource_spans.scope_spans[0].spans;

    let failed_span = spans
        .iter()
        .find(|s| s.name == "rein.step.failed")
        .expect("must have a rein.step.failed span");

    let kind_attr = failed_span
        .attributes
        .iter()
        .find(|a| a.key == "rein.step.error_kind")
        .expect("rein.step.failed span must have rein.step.error_kind attribute");

    assert_eq!(
        kind_attr.value.string_value.as_deref(),
        Some("agent_not_found"),
        "rein.step.error_kind must equal the error_kind field value; got: {:?}",
        kind_attr.value
    );
}

// Normal (non-partial) trace must NOT have rein.run.partial attribute.
#[test]
fn non_partial_trace_has_no_partial_attribute() {
    let trace = sample_trace();
    assert!(!trace.is_partial, "sample_trace() must not be partial");

    let resource_spans = to_otlp(&trace);
    let root = &resource_spans.scope_spans[0].spans[0];

    assert!(
        root.attributes.iter().all(|a| a.key != "rein.run.partial"),
        "non-partial trace must not have rein.run.partial; attributes: {:?}",
        root.attributes
    );
}
