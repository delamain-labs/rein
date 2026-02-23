use std::collections::HashMap;

use serde_json::json;

use super::*;

fn tool_call(ns: &str, action: &str) -> ToolCall {
    ToolCall {
        namespace: ns.to_string(),
        action: action.to_string(),
        arguments: json!({}),
    }
}

fn ctx<'a>(call: &'a ToolCall, secrets: &'a HashMap<String, String>) -> ToolCallContext<'a> {
    ToolCallContext {
        tool_call: call,
        secrets,
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

    let call = tool_call("zendesk", "read_ticket");
    let secrets = HashMap::new();
    let result = executor
        .execute(&ctx(&call, &secrets))
        .await
        .expect("should work");
    assert!(result.success);
    assert!(result.output.contains("123"));
}

#[tokio::test]
async fn mock_returns_failure() {
    let executor = MockExecutor::new();
    executor.on_call_fail("zendesk", "refund", "insufficient funds");

    let call = tool_call("zendesk", "refund");
    let secrets = HashMap::new();
    let err = executor.execute(&ctx(&call, &secrets)).await.unwrap_err();
    assert!(err.to_string().contains("insufficient funds"));
}

#[tokio::test]
async fn mock_unknown_tool_returns_not_found() {
    let executor = MockExecutor::new();
    let call = tool_call("stripe", "charge");
    let secrets = HashMap::new();
    let err = executor.execute(&ctx(&call, &secrets)).await.unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[tokio::test]
async fn mock_multiple_tools() {
    let executor = MockExecutor::new();
    executor.on_call("zendesk", "read_ticket", "ticket data");
    executor.on_call("zendesk", "reply_ticket", "reply sent");

    let call1 = tool_call("zendesk", "read_ticket");
    let call2 = tool_call("zendesk", "reply_ticket");
    let secrets = HashMap::new();
    let r1 = executor.execute(&ctx(&call1, &secrets)).await.expect("ok");
    let r2 = executor.execute(&ctx(&call2, &secrets)).await.expect("ok");
    assert!(r1.output.contains("ticket"));
    assert!(r2.output.contains("reply"));
}

#[tokio::test]
async fn execute_accepts_context_with_secrets() {
    // Verifies that execute() accepts a ToolCallContext carrying a non-empty
    // secrets map without panicking or returning an error. MockExecutor does
    // not inspect secrets; this test confirms the interface wiring compiles
    // and runs correctly end-to-end.
    let executor = MockExecutor::new();
    executor.on_call("api", "call", "ok");

    let call = tool_call("api", "call");
    let mut secrets = HashMap::new();
    secrets.insert("TOKEN".to_string(), "secret-value".to_string());
    let result = executor
        .execute(&ctx(&call, &secrets))
        .await
        .expect("executor should accept context carrying secrets");
    assert!(result.success);
    assert_eq!(result.output, "ok");
}
