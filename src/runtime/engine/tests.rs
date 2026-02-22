use serde_json::json;

use super::*;
use crate::ast::ValueExpr;
use crate::ast::{AgentDef, Capability, Span};
use crate::runtime::executor::MockExecutor;
use crate::runtime::provider::{ChatResponse, MockProvider, ToolCallRequest, ToolDef, Usage};

fn make_agent(
    can: Vec<Capability>,
    cannot: Vec<Capability>,
    budget_cents: Option<u64>,
) -> AgentDef {
    AgentDef {
        from: None,
        name: "test".to_string(),
        model: Some(ValueExpr::Literal("gpt-4o".into())),
        can,
        cannot,
        budget: budget_cents.map(|amount| crate::ast::Budget {
            amount,
            currency: "$".to_string(),
            unit: "per run".to_string(),
            span: Span { start: 0, end: 1 },
        }),
        guardrails: None,
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

fn simple_response(content: &str) -> ChatResponse {
    ChatResponse {
        content: content.to_string(),
        tool_calls: vec![],
        usage: Usage {
            input_tokens: 100,
            output_tokens: 50,
        },
        model: "gpt-4o".to_string(),
    }
}

fn tool_call_response(tool_name: &str, args: serde_json::Value) -> ChatResponse {
    ChatResponse {
        content: String::new(),
        tool_calls: vec![ToolCallRequest {
            id: "call_1".to_string(),
            name: tool_name.to_string(),
            arguments: args,
        }],
        usage: Usage {
            input_tokens: 100,
            output_tokens: 50,
        },
        model: "gpt-4o".to_string(),
    }
}

#[tokio::test]
async fn simple_response_no_tools() {
    let agent = make_agent(vec![], vec![], None);
    let registry = ToolRegistry::from_agent(&agent);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    provider.push_response(simple_response("Hello! How can I help?"));

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig::default(),
    );

    let result = engine.run("Hi").await.expect("should succeed");
    assert_eq!(result.response, "Hello! How can I help?");
    assert!(result.total_tokens > 0);
}

#[tokio::test]
async fn tool_call_then_response() {
    let agent = make_agent(vec![cap("zendesk", "read_ticket")], vec![], None);
    let registry = ToolRegistry::from_agent(&agent);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // First: LLM requests a tool call
    provider.push_response(tool_call_response(
        "zendesk.read_ticket",
        json!({"id": 123}),
    ));
    // Second: LLM responds with final answer
    provider.push_response(simple_response("Ticket #123 is about billing."));

    executor.on_call(
        "zendesk",
        "read_ticket",
        r#"{"id": 123, "subject": "Billing issue"}"#,
    );

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![ToolDef {
            name: "zendesk.read_ticket".to_string(),
            description: "Read a ticket".to_string(),
            parameters: json!({}),
        }],
        RunConfig::default(),
    );

    let result = engine
        .run("What's ticket 123?")
        .await
        .expect("should succeed");
    assert_eq!(result.response, "Ticket #123 is about billing.");

    // Should have 2 LLM calls and tool events in trace
    let llm_calls: Vec<_> = result
        .trace
        .events
        .iter()
        .filter(|e| matches!(e, RunEvent::LlmCall { .. }))
        .collect();
    assert_eq!(llm_calls.len(), 2);
}

#[tokio::test]
async fn denied_tool_sends_error_to_llm() {
    let agent = make_agent(vec![], vec![cap("zendesk", "delete_ticket")], None);
    let registry = ToolRegistry::from_agent(&agent);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // LLM tries to call a denied tool
    provider.push_response(tool_call_response("zendesk.delete_ticket", json!({})));
    // LLM responds after getting the denial
    provider.push_response(simple_response("I can't delete tickets."));

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig::default(),
    );

    let result = engine
        .run("Delete ticket 123")
        .await
        .expect("should succeed");
    assert_eq!(result.response, "I can't delete tickets.");

    // Verify there's a denied tool attempt
    let denied: Vec<_> = result
        .trace
        .events
        .iter()
        .filter(|e| matches!(e, RunEvent::ToolCallAttempt { allowed: false, .. }))
        .collect();
    assert_eq!(denied.len(), 1);
}

#[tokio::test]
async fn budget_exceeded_stops_run() {
    let agent = make_agent(vec![], vec![], Some(1)); // 1 cent budget
    let registry = ToolRegistry::from_agent(&agent);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // Response that will cost more than 1 cent
    provider.push_response(ChatResponse {
        content: String::new(),
        tool_calls: vec![],
        usage: Usage {
            input_tokens: 100_000,
            output_tokens: 50_000,
        }, // ~75 cents for gpt-4o
        model: "gpt-4o".to_string(),
    });

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig {
            budget_cents: 1,
            ..RunConfig::default()
        },
    );

    let err = engine.run("Hi").await.unwrap_err();
    assert!(matches!(err, RunError::BudgetExceeded));
}

#[tokio::test]
async fn max_turns_respected() {
    let agent = make_agent(vec![cap("zendesk", "read_ticket")], vec![], None);
    let registry = ToolRegistry::from_agent(&agent);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // Keep requesting tool calls to exhaust turns
    for _ in 0..3 {
        provider.push_response(tool_call_response("zendesk.read_ticket", json!({})));
    }
    executor.on_call("zendesk", "read_ticket", "data");

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig {
            max_turns: 3,
            ..RunConfig::default()
        },
    );

    let result = engine.run("loop").await.expect("should not error");
    assert_eq!(result.response, "Max turns reached");
}

#[tokio::test]
async fn system_prompt_included() {
    let agent = make_agent(vec![], vec![], None);
    let registry = ToolRegistry::from_agent(&agent);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    provider.push_response(simple_response("I am a support agent."));

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig {
            system_prompt: Some("You are a support agent.".to_string()),
            ..RunConfig::default()
        },
    );

    let result = engine.run("Who are you?").await.expect("should succeed");
    assert_eq!(result.response, "I am a support agent.");
}

#[tokio::test]
async fn trace_contains_run_complete() {
    let agent = make_agent(vec![], vec![], None);
    let registry = ToolRegistry::from_agent(&agent);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    provider.push_response(simple_response("Done"));

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig::default(),
    );

    let result = engine.run("test").await.expect("ok");
    let complete: Vec<_> = result
        .trace
        .events
        .iter()
        .filter(|e| matches!(e, RunEvent::RunComplete { .. }))
        .collect();
    assert_eq!(complete.len(), 1);
}

#[tokio::test]
async fn stream_callback_receives_text() {
    use std::sync::{Arc, Mutex};

    struct CollectStream {
        texts: Arc<Mutex<Vec<String>>>,
        completed: Arc<Mutex<bool>>,
    }

    impl super::StreamCallback for CollectStream {
        fn on_text(&self, text: &str) {
            self.texts.lock().unwrap().push(text.to_string());
        }
        fn on_tool_call(&self, _ns: &str, _action: &str) {}
        fn on_complete(&self) {
            *self.completed.lock().unwrap() = true;
        }
    }

    let provider = MockProvider::new();
    provider.push_response(ChatResponse {
        content: "Hello streamed!".to_string(),
        tool_calls: vec![],
        usage: Usage {
            input_tokens: 10,
            output_tokens: 5,
        },
        model: "gpt-4o".to_string(),
    });

    let executor = MockExecutor::new();
    let agent = make_agent(vec![], vec![], None);
    let registry = ToolRegistry::from_agent(&agent);
    let config = RunConfig::default();

    let texts = Arc::new(Mutex::new(Vec::new()));
    let completed = Arc::new(Mutex::new(false));
    let stream = CollectStream {
        texts: Arc::clone(&texts),
        completed: Arc::clone(&completed),
    };

    let engine = AgentEngine::new(&provider, &executor, &registry, vec![], config)
        .with_stream(Box::new(stream));
    let result = engine.run("Hi").await.unwrap();

    assert_eq!(result.response, "Hello streamed!");
    assert_eq!(texts.lock().unwrap().len(), 1);
    assert_eq!(texts.lock().unwrap()[0], "Hello streamed!");
    assert!(*completed.lock().unwrap());
}
