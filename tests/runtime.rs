/// Integration tests for the full agent runtime pipeline.
///
/// Tests the flow: parse .rein → build engine → run with mocks → verify trace.

use serde_json::json;

use rein::runtime::engine::{AgentEngine, RunConfig};
use rein::runtime::executor::MockExecutor;
use rein::runtime::permissions::ToolRegistry;
use rein::runtime::provider::{ChatResponse, MockProvider, ToolCallRequest, ToolDef, Usage};
use rein::runtime::{RunError, RunEvent};

fn simple_response(content: &str) -> ChatResponse {
    ChatResponse {
        content: content.to_string(),
        tool_calls: vec![],
        usage: Usage { input_tokens: 100, output_tokens: 50 },
        model: "gpt-4o".to_string(),
    }
}

fn tool_call_response(name: &str, args: serde_json::Value) -> ChatResponse {
    ChatResponse {
        content: String::new(),
        tool_calls: vec![ToolCallRequest {
            id: "call_1".to_string(),
            name: name.to_string(),
            arguments: args,
        }],
        usage: Usage { input_tokens: 100, output_tokens: 50 },
        model: "gpt-4o".to_string(),
    }
}

/// Parse a .rein file and build an engine from the first agent.
fn parse_and_build_registry(rein_source: &str) -> ToolRegistry {
    let file = rein::parser::parse(rein_source).expect("parse should succeed");
    let agent = &file.agents[0];
    ToolRegistry::from_agent(agent)
}

#[tokio::test]
async fn full_pipeline_simple_response() {
    let source = r#"
        agent test {
            model: openai
            can [ zendesk.read_ticket ]
            cannot [ zendesk.delete_ticket ]
            budget: $1 per request
        }
    "#;

    let registry = parse_and_build_registry(source);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    provider.push_response(simple_response("Hello from the agent!"));

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig { budget_cents: 100, ..RunConfig::default() },
    );

    let result = engine.run("Hi").await.expect("should succeed");
    assert_eq!(result.response, "Hello from the agent!");
    assert!(result.total_tokens > 0);
    assert!(result.total_cost_cents > 0);

    // Verify trace
    let json = result.trace.to_json().expect("valid JSON");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert!(parsed["events"].as_array().unwrap().len() >= 2);
}

#[tokio::test]
async fn full_pipeline_tool_call_allowed() {
    let source = r#"
        agent test {
            model: openai
            can [ zendesk.read_ticket ]
            cannot [ zendesk.delete_ticket ]
        }
    "#;

    let registry = parse_and_build_registry(source);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    provider.push_response(tool_call_response("zendesk.read_ticket", json!({"id": 42})));
    provider.push_response(simple_response("Ticket 42 is about billing."));
    executor.on_call("zendesk", "read_ticket", r#"{"id": 42, "subject": "Billing"}"#);

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

    let result = engine.run("What's ticket 42?").await.expect("should succeed");
    assert_eq!(result.response, "Ticket 42 is about billing.");

    // Verify tool was allowed in trace
    let allowed: Vec<_> = result.trace.events.iter().filter(|e| {
        matches!(e, RunEvent::ToolCallAttempt { allowed: true, .. })
    }).collect();
    assert_eq!(allowed.len(), 1);
}

#[tokio::test]
async fn full_pipeline_tool_call_denied() {
    let source = r#"
        agent test {
            model: openai
            can [ zendesk.read_ticket ]
            cannot [ zendesk.delete_ticket ]
        }
    "#;

    let registry = parse_and_build_registry(source);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    provider.push_response(tool_call_response("zendesk.delete_ticket", json!({})));
    provider.push_response(simple_response("I cannot delete tickets."));

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig::default(),
    );

    let result = engine.run("Delete ticket 42").await.expect("should succeed");
    assert_eq!(result.response, "I cannot delete tickets.");

    // Verify tool was denied in trace
    let denied: Vec<_> = result.trace.events.iter().filter(|e| {
        matches!(e, RunEvent::ToolCallAttempt { allowed: false, .. })
    }).collect();
    assert_eq!(denied.len(), 1);
}

#[tokio::test]
async fn full_pipeline_budget_exceeded() {
    let source = r#"
        agent test {
            model: openai
            can [ zendesk.read_ticket ]
            budget: $0.01 per request
        }
    "#;

    let registry = parse_and_build_registry(source);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // Response with high token usage to exceed 1-cent budget
    provider.push_response(ChatResponse {
        content: String::new(),
        tool_calls: vec![],
        usage: Usage { input_tokens: 100_000, output_tokens: 50_000 },
        model: "gpt-4o".to_string(),
    });

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig { budget_cents: 1, ..RunConfig::default() },
    );

    let err = engine.run("Hi").await.unwrap_err();
    assert!(matches!(err, RunError::BudgetExceeded));
}

#[tokio::test]
async fn full_pipeline_trace_summary_is_readable() {
    let source = r#"
        agent test {
            model: openai
            can [ search.web ]
        }
    "#;

    let registry = parse_and_build_registry(source);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    provider.push_response(tool_call_response("search.web", json!({"q": "rust"})));
    provider.push_response(simple_response("Found results about Rust."));
    executor.on_call("search", "web", "10 results for rust");

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![ToolDef {
            name: "search.web".to_string(),
            description: "Search the web".to_string(),
            parameters: json!({}),
        }],
        RunConfig::default(),
    );

    let result = engine.run("Search for Rust").await.expect("ok");
    let summary = result.trace.summary();

    assert!(summary.contains("turn 1"), "summary: {summary}");
    assert!(summary.contains("search.web"), "summary: {summary}");
    assert!(summary.contains("Done"), "summary: {summary}");
}
