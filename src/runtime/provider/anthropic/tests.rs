use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::AnthropicProvider;
use crate::runtime::provider::{Message, Provider, ToolDef};

fn mock_text_response(text: &str) -> serde_json::Value {
    json!({
        "id": "msg_test",
        "type": "message",
        "role": "assistant",
        "content": [{"type": "text", "text": text}],
        "model": "claude-sonnet-4-20250514",
        "usage": {"input_tokens": 50, "output_tokens": 25}
    })
}

fn mock_tool_response(tool_id: &str, tool_name: &str, input: serde_json::Value) -> serde_json::Value {
    json!({
        "id": "msg_test",
        "type": "message",
        "role": "assistant",
        "content": [
            {"type": "tool_use", "id": tool_id, "name": tool_name, "input": input}
        ],
        "model": "claude-sonnet-4-20250514",
        "usage": {"input_tokens": 100, "output_tokens": 50}
    })
}

#[tokio::test]
async fn basic_text_response() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "test-key"))
        .and(header("anthropic-version", "2023-06-01"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(mock_text_response("Hello!")),
        )
        .mount(&server)
        .await;

    let provider = AnthropicProvider::new("test-key", "claude-sonnet-4-20250514", Some(server.uri()), None);
    let resp = provider.chat(&[Message::user("Hi")], &[]).await.expect("ok");

    assert_eq!(resp.content, "Hello!");
    assert!(resp.tool_calls.is_empty());
    assert_eq!(resp.usage.input_tokens, 50);
    assert_eq!(resp.model, "claude-sonnet-4-20250514");
}

#[tokio::test]
async fn tool_use_response() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(
                mock_tool_response("toolu_1", "read_file", json!({"path": "/tmp/test"}))
            ),
        )
        .mount(&server)
        .await;

    let provider = AnthropicProvider::new("test-key", "claude-sonnet-4-20250514", Some(server.uri()), None);
    let tools = vec![ToolDef {
        name: "read_file".to_string(),
        description: "Read a file".to_string(),
        parameters: json!({"type": "object"}),
    }];
    let resp = provider.chat(&[Message::user("Read /tmp/test")], &tools).await.expect("ok");

    assert!(resp.content.is_empty());
    assert_eq!(resp.tool_calls.len(), 1);
    assert_eq!(resp.tool_calls[0].id, "toolu_1");
    assert_eq!(resp.tool_calls[0].name, "read_file");
}

#[tokio::test]
async fn auth_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "type": "error",
            "error": {"type": "authentication_error", "message": "invalid x-api-key"}
        })))
        .mount(&server)
        .await;

    let provider = AnthropicProvider::new("bad-key", "claude-sonnet-4-20250514", Some(server.uri()), None);
    let err = provider.chat(&[Message::user("Hi")], &[]).await.unwrap_err();
    assert!(err.to_string().contains("auth"), "got: {}", err);
}

#[tokio::test]
async fn rate_limit() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&server)
        .await;

    let provider = AnthropicProvider::new("key", "claude-sonnet-4-20250514", Some(server.uri()), None);
    let err = provider.chat(&[Message::user("Hi")], &[]).await.unwrap_err();
    assert_eq!(err.to_string(), "rate limited");
}

#[tokio::test]
async fn server_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "type": "error",
            "error": {"type": "api_error", "message": "overloaded"}
        })))
        .mount(&server)
        .await;

    let provider = AnthropicProvider::new("key", "claude-sonnet-4-20250514", Some(server.uri()), None);
    let err = provider.chat(&[Message::user("Hi")], &[]).await.unwrap_err();
    assert!(err.to_string().contains("overloaded"), "got: {}", err);
}

#[test]
fn provider_name_is_anthropic() {
    let provider = AnthropicProvider::new("key", "model", None, None);
    assert_eq!(provider.name(), "anthropic");
}
