use super::*;

#[test]
fn message_constructors() {
    let sys = Message::system("hello");
    assert_eq!(sys.role, Role::System);
    assert_eq!(sys.content, "hello");
    assert!(sys.tool_call_id.is_none());

    let user = Message::user("question");
    assert_eq!(user.role, Role::User);

    let asst = Message::assistant("answer");
    assert_eq!(asst.role, Role::Assistant);

    let tool = Message::tool("call-1", "result");
    assert_eq!(tool.role, Role::Tool);
    assert_eq!(tool.tool_call_id.as_deref(), Some("call-1"));
}

#[test]
fn tool_def_serializes() {
    let def = ToolDef {
        name: "read_file".to_string(),
        description: "Read a file".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" }
            }
        }),
    };
    let json = serde_json::to_string(&def).expect("serialize");
    assert!(json.contains("read_file"));
}

#[test]
fn usage_default() {
    let u = Usage::default();
    assert_eq!(u.input_tokens, 0);
    assert_eq!(u.output_tokens, 0);
}

#[tokio::test]
async fn mock_provider_returns_queued_response() {
    let mock = MockProvider::new();
    mock.push_response(ChatResponse {
        content: "Hello!".to_string(),
        tool_calls: vec![],
        usage: Usage { input_tokens: 10, output_tokens: 5 },
        model: "mock-1".to_string(),
    });

    let resp = mock.chat(&[Message::user("Hi")], &[]).await.expect("should succeed");
    assert_eq!(resp.content, "Hello!");
    assert_eq!(resp.usage.input_tokens, 10);
    assert_eq!(resp.model, "mock-1");
}

#[tokio::test]
async fn mock_provider_returns_queued_error() {
    let mock = MockProvider::new();
    mock.push_error("connection refused");

    let err = mock.chat(&[Message::user("Hi")], &[]).await.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("connection refused"), "got: {msg}");
}

#[tokio::test]
async fn mock_provider_empty_queue_errors() {
    let mock = MockProvider::new();
    let err = mock.chat(&[Message::user("Hi")], &[]).await.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("no mock responses"), "got: {msg}");
}

#[tokio::test]
async fn mock_provider_multiple_responses_in_order() {
    let mock = MockProvider::new();
    mock.push_response(ChatResponse {
        content: "first".to_string(),
        tool_calls: vec![],
        usage: Usage::default(),
        model: "mock-1".to_string(),
    });
    mock.push_response(ChatResponse {
        content: "second".to_string(),
        tool_calls: vec![],
        usage: Usage::default(),
        model: "mock-1".to_string(),
    });

    let r1 = mock.chat(&[Message::user("1")], &[]).await.expect("first");
    let r2 = mock.chat(&[Message::user("2")], &[]).await.expect("second");
    assert_eq!(r1.content, "first");
    assert_eq!(r2.content, "second");
}

#[test]
fn mock_provider_name() {
    let mock = MockProvider::new();
    assert_eq!(mock.name(), "mock");
}

#[test]
fn provider_error_display() {
    assert_eq!(ProviderError::Network("timeout".into()).to_string(), "network error: timeout");
    assert_eq!(
        ProviderError::Api { status: 429, body: "slow down".into() }.to_string(),
        "API error (429): slow down"
    );
    assert_eq!(ProviderError::Parse("bad json".into()).to_string(), "parse error: bad json");
    assert_eq!(ProviderError::Auth("invalid key".into()).to_string(), "auth error: invalid key");
    assert_eq!(ProviderError::RateLimited.to_string(), "rate limited");
}

#[test]
fn chat_response_with_tool_calls() {
    let resp = ChatResponse {
        content: String::new(),
        tool_calls: vec![
            ToolCallRequest {
                id: "call-1".to_string(),
                name: "read_file".to_string(),
                arguments: serde_json::json!({"path": "/tmp/test"}),
            },
        ],
        usage: Usage { input_tokens: 100, output_tokens: 50 },
        model: "gpt-4".to_string(),
    };
    assert!(resp.content.is_empty());
    assert_eq!(resp.tool_calls.len(), 1);
    assert_eq!(resp.tool_calls[0].name, "read_file");
}
