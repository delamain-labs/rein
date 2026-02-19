use super::*;

// ── ToolCall serialization ─────────────────────────────────────────────────

#[test]
fn tool_call_roundtrips() {
    let call = ToolCall {
        namespace: "zendesk".into(),
        action: "read_ticket".into(),
        arguments: serde_json::json!({ "id": 42 }),
    };
    let json = serde_json::to_string(&call).expect("serialize");
    let back: ToolCall = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.namespace, call.namespace);
    assert_eq!(back.action, call.action);
    assert_eq!(back.arguments, call.arguments);
}

#[test]
fn tool_call_serializes_to_expected_keys() {
    let call = ToolCall {
        namespace: "stripe".into(),
        action: "charge".into(),
        arguments: serde_json::Value::Null,
    };
    let v: serde_json::Value = serde_json::to_value(&call).expect("serialize");
    assert_eq!(v["namespace"], "stripe");
    assert_eq!(v["action"], "charge");
    assert!(v.get("arguments").is_some());
}

// ── ToolResult serialization ───────────────────────────────────────────────

#[test]
fn tool_result_roundtrips() {
    let result = ToolResult {
        success: true,
        output: "ticket #42 fetched".into(),
    };
    let json = serde_json::to_string(&result).expect("serialize");
    let back: ToolResult = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.success, result.success);
    assert_eq!(back.output, result.output);
}

#[test]
fn tool_result_failure_roundtrips() {
    let result = ToolResult {
        success: false,
        output: "not found".into(),
    };
    let json = serde_json::to_string(&result).expect("serialize");
    let back: ToolResult = serde_json::from_str(&json).expect("deserialize");
    assert!(!back.success);
    assert_eq!(back.output, "not found");
}

// ── RunEvent serialization ─────────────────────────────────────────────────

#[test]
fn run_event_llm_call_roundtrips() {
    let event = RunEvent::LlmCall {
        model: "claude-sonnet-4-6".into(),
        input_tokens: 1024,
        output_tokens: 256,
        cost_cents: 3,
    };
    let json = serde_json::to_string(&event).expect("serialize");
    let back: RunEvent = serde_json::from_str(&json).expect("deserialize");
    let RunEvent::LlmCall {
        model,
        input_tokens,
        output_tokens,
        cost_cents,
    } = back
    else {
        panic!("wrong variant");
    };
    assert_eq!(model, "claude-sonnet-4-6");
    assert_eq!(input_tokens, 1024);
    assert_eq!(output_tokens, 256);
    assert_eq!(cost_cents, 3);
}

#[test]
fn run_event_llm_call_has_type_tag() {
    let event = RunEvent::LlmCall {
        model: "claude-opus-4-6".into(),
        input_tokens: 100,
        output_tokens: 50,
        cost_cents: 1,
    };
    let v: serde_json::Value = serde_json::to_value(&event).expect("serialize");
    assert_eq!(v["type"], "llm_call");
}

#[test]
fn run_event_tool_call_attempt_allowed_roundtrips() {
    let event = RunEvent::ToolCallAttempt {
        tool: ToolCall {
            namespace: "zendesk".into(),
            action: "read_ticket".into(),
            arguments: serde_json::Value::Null,
        },
        allowed: true,
        reason: None,
    };
    let json = serde_json::to_string(&event).expect("serialize");
    let back: RunEvent = serde_json::from_str(&json).expect("deserialize");
    let RunEvent::ToolCallAttempt {
        allowed, reason, ..
    } = back
    else {
        panic!("wrong variant");
    };
    assert!(allowed);
    assert!(reason.is_none());
}

#[test]
fn run_event_tool_call_attempt_denied_with_reason() {
    let event = RunEvent::ToolCallAttempt {
        tool: ToolCall {
            namespace: "stripe".into(),
            action: "charge".into(),
            arguments: serde_json::json!({ "amount": 9999 }),
        },
        allowed: false,
        reason: Some("exceeds per-call budget".into()),
    };
    let json = serde_json::to_string(&event).expect("serialize");
    let back: RunEvent = serde_json::from_str(&json).expect("deserialize");
    let RunEvent::ToolCallAttempt {
        allowed, reason, ..
    } = back
    else {
        panic!("wrong variant");
    };
    assert!(!allowed);
    assert_eq!(reason.as_deref(), Some("exceeds per-call budget"));
}

#[test]
fn run_event_tool_call_attempt_has_type_tag() {
    let event = RunEvent::ToolCallAttempt {
        tool: ToolCall {
            namespace: "ns".into(),
            action: "act".into(),
            arguments: serde_json::Value::Null,
        },
        allowed: true,
        reason: None,
    };
    let v: serde_json::Value = serde_json::to_value(&event).expect("serialize");
    assert_eq!(v["type"], "tool_call_attempt");
}

#[test]
fn run_event_tool_call_result_roundtrips() {
    let event = RunEvent::ToolCallResult {
        tool: ToolCall {
            namespace: "zendesk".into(),
            action: "read_ticket".into(),
            arguments: serde_json::Value::Null,
        },
        result: ToolResult {
            success: true,
            output: "ok".into(),
        },
    };
    let json = serde_json::to_string(&event).expect("serialize");
    let back: RunEvent = serde_json::from_str(&json).expect("deserialize");
    let RunEvent::ToolCallResult { tool, result } = back else {
        panic!("wrong variant");
    };
    assert_eq!(tool.namespace, "zendesk");
    assert!(result.success);
}

#[test]
fn run_event_budget_update_roundtrips() {
    let event = RunEvent::BudgetUpdate {
        spent_cents: 50,
        limit_cents: 300,
    };
    let json = serde_json::to_string(&event).expect("serialize");
    let back: RunEvent = serde_json::from_str(&json).expect("deserialize");
    let RunEvent::BudgetUpdate {
        spent_cents,
        limit_cents,
    } = back
    else {
        panic!("wrong variant");
    };
    assert_eq!(spent_cents, 50);
    assert_eq!(limit_cents, 300);
}

#[test]
fn run_event_budget_update_has_type_tag() {
    let event = RunEvent::BudgetUpdate {
        spent_cents: 1,
        limit_cents: 100,
    };
    let v: serde_json::Value = serde_json::to_value(&event).expect("serialize");
    assert_eq!(v["type"], "budget_update");
}

#[test]
fn run_event_run_complete_roundtrips() {
    let event = RunEvent::RunComplete {
        total_cost_cents: 99,
        total_tokens: 2048,
    };
    let json = serde_json::to_string(&event).expect("serialize");
    let back: RunEvent = serde_json::from_str(&json).expect("deserialize");
    let RunEvent::RunComplete {
        total_cost_cents,
        total_tokens,
    } = back
    else {
        panic!("wrong variant");
    };
    assert_eq!(total_cost_cents, 99);
    assert_eq!(total_tokens, 2048);
}

#[test]
fn run_event_run_complete_has_type_tag() {
    let event = RunEvent::RunComplete {
        total_cost_cents: 0,
        total_tokens: 0,
    };
    let v: serde_json::Value = serde_json::to_value(&event).expect("serialize");
    assert_eq!(v["type"], "run_complete");
}

// ── RunTrace serialization ─────────────────────────────────────────────────

#[test]
fn run_trace_empty_roundtrips() {
    let trace = RunTrace { events: vec![] };
    let json = serde_json::to_string(&trace).expect("serialize");
    let back: RunTrace = serde_json::from_str(&json).expect("deserialize");
    assert!(back.events.is_empty());
}

#[test]
fn run_trace_with_events_roundtrips() {
    let trace = RunTrace {
        events: vec![
            RunEvent::LlmCall {
                model: "claude-haiku-4-5-20251001".into(),
                input_tokens: 512,
                output_tokens: 128,
                cost_cents: 1,
            },
            RunEvent::BudgetUpdate {
                spent_cents: 1,
                limit_cents: 1000,
            },
            RunEvent::RunComplete {
                total_cost_cents: 1,
                total_tokens: 640,
            },
        ],
    };
    let json = serde_json::to_string(&trace).expect("serialize");
    let back: RunTrace = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.events.len(), 3);
}

// ── RunError serialization ─────────────────────────────────────────────────

#[test]
fn run_error_budget_exceeded_roundtrips() {
    let err = RunError::BudgetExceeded;
    let json = serde_json::to_string(&err).expect("serialize");
    let back: RunError = serde_json::from_str(&json).expect("deserialize");
    assert!(matches!(back, RunError::BudgetExceeded));
}

#[test]
fn run_error_permission_denied_roundtrips() {
    let err = RunError::PermissionDenied;
    let json = serde_json::to_string(&err).expect("serialize");
    let back: RunError = serde_json::from_str(&json).expect("deserialize");
    assert!(matches!(back, RunError::PermissionDenied));
}

#[test]
fn run_error_provider_error_roundtrips() {
    let err = RunError::ProviderError;
    let json = serde_json::to_string(&err).expect("serialize");
    let back: RunError = serde_json::from_str(&json).expect("deserialize");
    assert!(matches!(back, RunError::ProviderError));
}

#[test]
fn run_error_config_error_roundtrips() {
    let err = RunError::ConfigError;
    let json = serde_json::to_string(&err).expect("serialize");
    let back: RunError = serde_json::from_str(&json).expect("deserialize");
    assert!(matches!(back, RunError::ConfigError));
}

#[test]
fn run_error_serializes_as_snake_case() {
    let v: serde_json::Value = serde_json::to_value(RunError::BudgetExceeded).expect("serialize");
    assert_eq!(v, "budget_exceeded");

    let v: serde_json::Value = serde_json::to_value(RunError::PermissionDenied).expect("serialize");
    assert_eq!(v, "permission_denied");

    let v: serde_json::Value = serde_json::to_value(RunError::ProviderError).expect("serialize");
    assert_eq!(v, "provider_error");

    let v: serde_json::Value = serde_json::to_value(RunError::ConfigError).expect("serialize");
    assert_eq!(v, "config_error");
}
