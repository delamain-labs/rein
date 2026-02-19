use serde_json::json;

use super::*;
use crate::ast::ValueExpr;
use crate::ast::{AgentDef, Capability, Constraint, Span};

fn make_agent(can: Vec<Capability>, cannot: Vec<Capability>) -> AgentDef {
    AgentDef {
        name: "test_agent".to_string(),
        model: Some(ValueExpr::Literal("openai".into())),
        can,
        cannot,
        budget: None,
        span: Span { start: 0, end: 10 },
    }
}

fn cap(ns: &str, action: &str) -> Capability {
    Capability {
        namespace: ns.to_string(),
        action: action.to_string(),
        constraint: None,
        span: Span { start: 0, end: 1 },
    }
}

fn cap_with_money(ns: &str, action: &str, amount: u64) -> Capability {
    Capability {
        namespace: ns.to_string(),
        action: action.to_string(),
        constraint: Some(Constraint::MonetaryCap {
            amount,
            currency: "$".to_string(),
        }),
        span: Span { start: 0, end: 1 },
    }
}

fn tool_call(ns: &str, action: &str) -> ToolCall {
    ToolCall {
        namespace: ns.to_string(),
        action: action.to_string(),
        arguments: json!({}),
    }
}

#[test]
fn allowed_tool_returns_allowed() {
    let agent = make_agent(vec![cap("zendesk", "read_ticket")], vec![]);
    let registry = ToolRegistry::from_agent(&agent);
    let interceptor = ToolInterceptor::new(&registry);

    assert_eq!(
        interceptor.intercept(&tool_call("zendesk", "read_ticket")),
        InterceptResult::Allowed
    );
}

#[test]
fn denied_tool_returns_denied() {
    let agent = make_agent(vec![], vec![cap("zendesk", "delete_ticket")]);
    let registry = ToolRegistry::from_agent(&agent);
    let interceptor = ToolInterceptor::new(&registry);

    let result = interceptor.intercept(&tool_call("zendesk", "delete_ticket"));
    match result {
        InterceptResult::Denied { reason } => {
            assert!(reason.contains("cannot"), "reason: {reason}");
        }
        other => panic!("expected Denied, got {other:?}"),
    }
}

#[test]
fn unknown_tool_is_denied_by_default() {
    let agent = make_agent(vec![cap("zendesk", "read_ticket")], vec![]);
    let registry = ToolRegistry::from_agent(&agent);
    let interceptor = ToolInterceptor::new(&registry);

    let result = interceptor.intercept(&tool_call("stripe", "charge"));
    match result {
        InterceptResult::Denied { reason } => {
            assert!(reason.contains("not in the can list"), "reason: {reason}");
        }
        other => panic!("expected Denied, got {other:?}"),
    }
}

#[test]
fn capped_tool_returns_capped_at() {
    let agent = make_agent(vec![cap_with_money("zendesk", "refund", 5000)], vec![]);
    let registry = ToolRegistry::from_agent(&agent);
    let interceptor = ToolInterceptor::new(&registry);

    assert_eq!(
        interceptor.intercept(&tool_call("zendesk", "refund")),
        InterceptResult::CappedAt {
            max_cents: 5000,
            currency: "$".to_string(),
        }
    );
}

#[test]
fn cannot_overrides_can() {
    let agent = make_agent(
        vec![cap("zendesk", "refund")],
        vec![cap("zendesk", "refund")],
    );
    let registry = ToolRegistry::from_agent(&agent);
    let interceptor = ToolInterceptor::new(&registry);

    let result = interceptor.intercept(&tool_call("zendesk", "refund"));
    assert!(matches!(result, InterceptResult::Denied { .. }));
}

#[test]
fn multiple_tools_mixed_permissions() {
    let agent = make_agent(
        vec![
            cap("zendesk", "read_ticket"),
            cap_with_money("zendesk", "refund", 5000),
        ],
        vec![cap("zendesk", "delete_ticket")],
    );
    let registry = ToolRegistry::from_agent(&agent);
    let interceptor = ToolInterceptor::new(&registry);

    assert_eq!(
        interceptor.intercept(&tool_call("zendesk", "read_ticket")),
        InterceptResult::Allowed
    );
    assert!(matches!(
        interceptor.intercept(&tool_call("zendesk", "refund")),
        InterceptResult::CappedAt {
            max_cents: 5000,
            ..
        }
    ));
    assert!(matches!(
        interceptor.intercept(&tool_call("zendesk", "delete_ticket")),
        InterceptResult::Denied { .. }
    ));
    assert!(matches!(
        interceptor.intercept(&tool_call("slack", "send_message")),
        InterceptResult::Denied { .. }
    ));
}
