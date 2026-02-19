use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::OpenAiProvider;
use crate::runtime::provider::{Message, Provider, ToolDef};

fn mock_response(content: &str, tool_calls: serde_json::Value) -> serde_json::Value {
    json!({
        "id": "chatcmpl-test",
        "object": "chat.completion",
        "model": "gpt-4o-2025-01-01",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": content,
                "tool_calls": tool_calls
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 25,
            "completion_tokens": 10,
            "total_tokens": 35
        }
    })
}

#[tokio::test]
async fn basic_chat_completion() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("Authorization", "Bearer test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_response("Hello!", json!([]))))
        .mount(&server)
        .await;

    let provider = OpenAiProvider::new("test-key", "gpt-4o", Some(server.uri()));
    let resp = provider
        .chat(&[Message::user("Hi")], &[])
        .await
        .expect("should succeed");

    assert_eq!(resp.content, "Hello!");
    assert!(resp.tool_calls.is_empty());
    assert_eq!(resp.usage.input_tokens, 25);
    assert_eq!(resp.usage.output_tokens, 10);
    assert_eq!(resp.model, "gpt-4o-2025-01-01");
}

#[tokio::test]
async fn chat_with_tool_calls() {
    let server = MockServer::start().await;

    let tool_calls = json!([{
        "id": "call_abc",
        "type": "function",
        "function": {
            "name": "read_file",
            "arguments": "{\"path\":\"/tmp/test\"}"
        }
    }]);

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(mock_response("", tool_calls)))
        .mount(&server)
        .await;

    let tools = vec![ToolDef {
        name: "read_file".to_string(),
        description: "Read a file".to_string(),
        parameters: json!({"type": "object", "properties": {"path": {"type": "string"}}}),
    }];

    let provider = OpenAiProvider::new("test-key", "gpt-4o", Some(server.uri()));
    let resp = provider
        .chat(&[Message::user("Read /tmp/test")], &tools)
        .await
        .expect("should succeed");

    assert!(resp.content.is_empty());
    assert_eq!(resp.tool_calls.len(), 1);
    assert_eq!(resp.tool_calls[0].id, "call_abc");
    assert_eq!(resp.tool_calls[0].name, "read_file");
    assert_eq!(resp.tool_calls[0].arguments["path"], "/tmp/test");
}

#[tokio::test]
async fn auth_error_returns_auth() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": { "message": "Invalid API key", "type": "invalid_request_error" }
        })))
        .mount(&server)
        .await;

    let provider = OpenAiProvider::new("bad-key", "gpt-4o", Some(server.uri()));
    let err = provider
        .chat(&[Message::user("Hi")], &[])
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("auth error"), "got: {msg}");
}

#[tokio::test]
async fn rate_limit_returns_rate_limited() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(429).set_body_json(json!({
            "error": { "message": "Rate limit exceeded" }
        })))
        .mount(&server)
        .await;

    let provider = OpenAiProvider::new("test-key", "gpt-4o", Some(server.uri()));
    let err = provider
        .chat(&[Message::user("Hi")], &[])
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert_eq!(msg, "rate limited");
}

#[tokio::test]
async fn server_error_returns_api_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "error": { "message": "Internal server error" }
        })))
        .mount(&server)
        .await;

    let provider = OpenAiProvider::new("test-key", "gpt-4o", Some(server.uri()));
    let err = provider
        .chat(&[Message::user("Hi")], &[])
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("API error (500)"), "got: {msg}");
    assert!(msg.contains("Internal server error"), "got: {msg}");
}

#[tokio::test]
async fn empty_choices_returns_parse_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "test",
            "object": "chat.completion",
            "model": "gpt-4o",
            "choices": [],
            "usage": { "prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0 }
        })))
        .mount(&server)
        .await;

    let provider = OpenAiProvider::new("test-key", "gpt-4o", Some(server.uri()));
    let err = provider
        .chat(&[Message::user("Hi")], &[])
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("no choices"), "got: {msg}");
}

#[test]
fn provider_name_is_openai() {
    let provider = OpenAiProvider::new("key", "model", None);
    assert_eq!(provider.name(), "openai");
}
