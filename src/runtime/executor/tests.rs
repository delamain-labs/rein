use serde_json::json;

use super::*;

fn tool_call(ns: &str, action: &str) -> ToolCall {
    ToolCall {
        namespace: ns.to_string(),
        action: action.to_string(),
        arguments: json!({}),
    }
}

#[tokio::test]
async fn mock_returns_registered_response() {
    let executor = MockExecutor::new();
    executor.on_call(
        "zendesk",
        "read_ticket",
        r#"{"id": 123, "subject": "Help"}"#,
    );

    let result = executor
        .execute(&tool_call("zendesk", "read_ticket"))
        .await
        .expect("should work");
    assert!(result.success);
    assert!(result.output.contains("123"));
}

#[tokio::test]
async fn mock_returns_failure() {
    let executor = MockExecutor::new();
    executor.on_call_fail("zendesk", "refund", "insufficient funds");

    let err = executor
        .execute(&tool_call("zendesk", "refund"))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("insufficient funds"));
}

#[tokio::test]
async fn mock_unknown_tool_returns_not_found() {
    let executor = MockExecutor::new();
    let err = executor
        .execute(&tool_call("stripe", "charge"))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[tokio::test]
async fn mock_multiple_tools() {
    let executor = MockExecutor::new();
    executor.on_call("zendesk", "read_ticket", "ticket data");
    executor.on_call("zendesk", "reply_ticket", "reply sent");

    let r1 = executor
        .execute(&tool_call("zendesk", "read_ticket"))
        .await
        .expect("ok");
    let r2 = executor
        .execute(&tool_call("zendesk", "reply_ticket"))
        .await
        .expect("ok");
    assert!(r1.output.contains("ticket"));
    assert!(r2.output.contains("reply"));
}
