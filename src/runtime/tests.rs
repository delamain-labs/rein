use super::*;

// ── ToolRegistry / permissions ─────────────────────────────────────────────

mod permissions_tests {
    use crate::ast::{AgentDef, Capability, Constraint, Span};
    use crate::runtime::permissions::{MonetaryCap, PermissionDenied, ToolRegistry};

    fn span() -> Span {
        Span::new(0, 1)
    }

    fn cap(namespace: &str, action: &str) -> Capability {
        Capability {
            namespace: namespace.into(),
            action: action.into(),
            constraint: None,
            span: span(),
        }
    }

    fn cap_with_monetary(namespace: &str, action: &str, amount: u64, currency: &str) -> Capability {
        Capability {
            namespace: namespace.into(),
            action: action.into(),
            constraint: Some(Constraint::MonetaryCap {
                amount,
                currency: currency.into(),
            }),
            span: span(),
        }
    }

    fn agent(can: Vec<Capability>, cannot: Vec<Capability>) -> AgentDef {
        AgentDef {
            name: "test_agent".into(),
            model: None,
            can,
            cannot,
            budget: None,
            guardrails: None,
            span: span(),
        }
    }

    #[test]
    fn allowed_tool_passes() {
        let registry =
            ToolRegistry::from_agent(&agent(vec![cap("zendesk", "read_ticket")], vec![]));
        assert!(registry.check_permission("zendesk", "read_ticket").is_ok());
    }

    #[test]
    fn denied_tool_is_blocked() {
        let registry =
            ToolRegistry::from_agent(&agent(vec![], vec![cap("zendesk", "delete_ticket")]));
        let err = registry
            .check_permission("zendesk", "delete_ticket")
            .unwrap_err();
        assert!(
            err.reason.contains("cannot"),
            "expected 'cannot' in reason, got: {}",
            err.reason
        );
    }

    #[test]
    fn unknown_tool_is_default_denied() {
        let registry =
            ToolRegistry::from_agent(&agent(vec![cap("zendesk", "read_ticket")], vec![]));
        let err = registry
            .check_permission("zendesk", "delete_ticket")
            .unwrap_err();
        assert!(
            err.reason.contains("default deny"),
            "expected 'default deny' in reason, got: {}",
            err.reason
        );
    }

    #[test]
    fn empty_agent_denies_all() {
        let registry = ToolRegistry::from_agent(&agent(vec![], vec![]));
        let err = registry.check_permission("any", "tool").unwrap_err();
        assert!(err.reason.contains("default deny"));
    }

    #[test]
    fn monetary_cap_is_tracked() {
        let registry = ToolRegistry::from_agent(&agent(
            vec![cap_with_monetary("zendesk", "refund", 5000, "USD")],
            vec![],
        ));
        assert!(registry.check_permission("zendesk", "refund").is_ok());
        let mc = registry
            .monetary_cap("zendesk", "refund")
            .expect("cap present");
        assert_eq!(mc.amount, 5000);
        assert_eq!(mc.currency, "USD");
    }

    #[test]
    fn unconstrained_tool_has_no_monetary_cap() {
        let registry =
            ToolRegistry::from_agent(&agent(vec![cap("zendesk", "read_ticket")], vec![]));
        assert!(registry.monetary_cap("zendesk", "read_ticket").is_none());
    }

    #[test]
    fn cannot_overrides_can_for_same_tool() {
        let registry = ToolRegistry::from_agent(&agent(
            vec![cap("zendesk", "read_ticket")],
            vec![cap("zendesk", "read_ticket")],
        ));
        let err = registry
            .check_permission("zendesk", "read_ticket")
            .unwrap_err();
        assert!(err.reason.contains("cannot"));
    }

    #[test]
    fn monetary_cap_absent_for_denied_tool() {
        let registry = ToolRegistry::from_agent(&agent(
            vec![],
            vec![cap_with_monetary("stripe", "charge", 1000, "USD")],
        ));
        assert!(registry.monetary_cap("stripe", "charge").is_none());
    }

    #[test]
    fn multiple_can_tools_all_allowed() {
        let registry = ToolRegistry::from_agent(&agent(
            vec![
                cap("zendesk", "read_ticket"),
                cap("zendesk", "reply_ticket"),
                cap("stripe", "read_charge"),
            ],
            vec![],
        ));
        assert!(registry.check_permission("zendesk", "read_ticket").is_ok());
        assert!(registry.check_permission("zendesk", "reply_ticket").is_ok());
        assert!(registry.check_permission("stripe", "read_charge").is_ok());
        assert!(
            registry
                .check_permission("stripe", "delete_charge")
                .is_err()
        );
    }

    #[test]
    fn permission_denied_display_contains_reason() {
        let denied = PermissionDenied {
            reason: "test reason".into(),
        };
        let s = denied.to_string();
        assert!(s.contains("test reason"), "got: {s}");
    }

    #[test]
    fn permission_denied_implements_error() {
        fn accepts_error(_: &dyn std::error::Error) {}
        let denied = PermissionDenied {
            reason: "boom".into(),
        };
        accepts_error(&denied);
    }

    #[test]
    fn monetary_cap_partial_eq() {
        let a = MonetaryCap {
            amount: 100,
            currency: "USD".into(),
        };
        let b = MonetaryCap {
            amount: 100,
            currency: "USD".into(),
        };
        assert_eq!(a, b);
    }
}

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

// ── RunTrace output ────────────────────────────────────────────────────────

#[test]
fn trace_to_json_produces_valid_json() {
    let trace = RunTrace {
        events: vec![
            RunEvent::LlmCall {
                model: "gpt-4o".into(),
                input_tokens: 100,
                output_tokens: 50,
                cost_cents: 1,
            },
            RunEvent::RunComplete {
                total_cost_cents: 1,
                total_tokens: 150,
            },
        ],
    };
    let json = trace.to_json().expect("should serialize");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
    assert!(parsed["events"].is_array());
    assert_eq!(parsed["events"].as_array().unwrap().len(), 2);
}

#[test]
fn trace_summary_contains_turns() {
    let trace = RunTrace {
        events: vec![
            RunEvent::LlmCall {
                model: "gpt-4o".into(),
                input_tokens: 100,
                output_tokens: 50,
                cost_cents: 1,
            },
            RunEvent::ToolCallAttempt {
                tool: ToolCall {
                    namespace: "zendesk".into(),
                    action: "read_ticket".into(),
                    arguments: serde_json::Value::Null,
                },
                allowed: true,
                reason: None,
            },
            RunEvent::ToolCallResult {
                tool: ToolCall {
                    namespace: "zendesk".into(),
                    action: "read_ticket".into(),
                    arguments: serde_json::Value::Null,
                },
                result: ToolResult {
                    success: true,
                    output: "ticket data".into(),
                },
            },
            RunEvent::RunComplete {
                total_cost_cents: 1,
                total_tokens: 150,
            },
        ],
    };
    let summary = trace.summary();
    assert!(summary.contains("turn 1"), "summary: {summary}");
    assert!(
        summary.contains("zendesk.read_ticket"),
        "summary: {summary}"
    );
    assert!(summary.contains("Done"), "summary: {summary}");
}

#[test]
fn trace_summary_shows_denied_tools() {
    let trace = RunTrace {
        events: vec![
            RunEvent::LlmCall {
                model: "gpt-4o".into(),
                input_tokens: 50,
                output_tokens: 25,
                cost_cents: 1,
            },
            RunEvent::ToolCallAttempt {
                tool: ToolCall {
                    namespace: "stripe".into(),
                    action: "charge".into(),
                    arguments: serde_json::Value::Null,
                },
                allowed: false,
                reason: Some("not in can list".into()),
            },
            RunEvent::RunComplete {
                total_cost_cents: 1,
                total_tokens: 75,
            },
        ],
    };
    let summary = trace.summary();
    assert!(summary.contains("✗"), "should show denied marker");
    assert!(summary.contains("not in can list"), "summary: {summary}");
}

#[test]
fn trace_summary_empty_trace() {
    let trace = RunTrace { events: vec![] };
    let summary = trace.summary();
    assert!(summary.is_empty());
}
