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

fn ctx<'a>(call: &'a ToolCall, secrets: &'a Secrets) -> ToolCallContext<'a> {
    ToolCallContext {
        tool_call: call,
        secrets,
    }
}

#[test]
fn secrets_debug_redacts_values() {
    let mut map = HashMap::new();
    map.insert("TOKEN".to_string(), "super-secret".to_string());
    map.insert("KEY".to_string(), "another-secret".to_string());
    let secrets = Secrets::from(map);
    let debug_output = format!("{secrets:?}");
    assert!(
        debug_output.contains("redacted"),
        "debug output must contain 'redacted', got: {debug_output}"
    );
    assert!(
        !debug_output.contains("super-secret"),
        "debug output must not reveal secret values, got: {debug_output}"
    );
    assert!(
        !debug_output.contains("another-secret"),
        "debug output must not reveal secret values, got: {debug_output}"
    );
    assert!(
        debug_output.contains('2'),
        "debug output must show key count, got: {debug_output}"
    );
}

#[test]
fn secrets_get_returns_value() {
    let mut map = HashMap::new();
    map.insert("TOKEN".to_string(), "value".to_string());
    let secrets = Secrets::from(map);
    assert_eq!(secrets.get("TOKEN"), Some(&"value".to_string()));
    assert_eq!(secrets.get("MISSING"), None);
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
    let secrets = Secrets::from(HashMap::new());
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
    let secrets = Secrets::from(HashMap::new());
    let err = executor.execute(&ctx(&call, &secrets)).await.unwrap_err();
    assert!(err.to_string().contains("insufficient funds"));
}

#[tokio::test]
async fn mock_unknown_tool_returns_not_found() {
    let executor = MockExecutor::new();
    let call = tool_call("stripe", "charge");
    let secrets = Secrets::from(HashMap::new());
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
    let secrets = Secrets::from(HashMap::new());
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
    let mut map = HashMap::new();
    map.insert("TOKEN".to_string(), "secret-value".to_string());
    let secrets = Secrets::from(map);
    let result = executor
        .execute(&ctx(&call, &secrets))
        .await
        .expect("executor should accept context carrying secrets");
    assert!(result.success);
    assert_eq!(result.output, "ok");
}
