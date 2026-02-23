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

    let trace = RunTrace { events };
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

#[test]
// #329: root span must have real timestamps derived from started_at/completed_at,
// not hardcoded zeros. sample_trace uses started_at = "2026-01-01T00:00:00Z".
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
    // end = start + duration_ms (1000ms) * 1_000_000
    assert_eq!(
        root.end_time_unix_nano,
        expected_start_ns + 1_000 * 1_000_000,
        "root span end must equal start + duration"
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
