use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::json;

use super::*;
use crate::ast::ValueExpr;
use crate::ast::{AgentDef, Capability, Span};
use crate::runtime::executor::MockExecutor;
use crate::runtime::otel_export::OtelMode;
use crate::runtime::policy::PolicyEngine;
use crate::runtime::provider::{ChatResponse, MockProvider, ToolCallRequest, ToolDef, Usage};

/// A `Provider` that never resolves — used to test timeout behaviour.
/// Tokio's mock clock (`start_paused = true`) advances automatically past any
/// `tokio::time::timeout` wrapper so the test finishes instantly.
struct HangingProvider;

/// A `Provider` that returns a tool-call on the first call, then hangs forever.
/// Used to test multi-turn timeout scenarios (#424).
struct HangAfterFirstProvider {
    calls: std::sync::Mutex<u32>,
}

impl HangAfterFirstProvider {
    fn new() -> Self {
        Self {
            calls: std::sync::Mutex::new(0),
        }
    }
}

#[async_trait::async_trait]
impl crate::runtime::provider::Provider for HangAfterFirstProvider {
    fn name(&self) -> &'static str {
        "hang_after_first"
    }

    async fn chat(
        &self,
        _messages: &[crate::runtime::provider::Message],
        _tools: &[ToolDef],
    ) -> Result<ChatResponse, crate::runtime::provider::ProviderError> {
        let call_num = {
            let mut guard = self.calls.lock().unwrap();
            let n = *guard;
            *guard += 1;
            n
        };
        if call_num == 0 {
            Ok(tool_call_response("test.noop", serde_json::json!({})))
        } else {
            std::future::pending().await
        }
    }
}

#[async_trait::async_trait]
impl crate::runtime::provider::Provider for HangingProvider {
    fn name(&self) -> &'static str {
        "hanging"
    }
    async fn chat(
        &self,
        _messages: &[crate::runtime::provider::Message],
        _tools: &[ToolDef],
    ) -> Result<ChatResponse, crate::runtime::provider::ProviderError> {
        std::future::pending().await
    }
}

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
    assert!(matches!(err, RunError::BudgetExceeded { .. }));
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

// --- #302 PolicyEngine Tests ---

fn make_policy_engine_with_threshold(metric: &str, threshold: f64) -> PolicyEngine {
    use crate::ast::{CompareOp, PolicyDef, PolicyTier, Span, WhenComparison, WhenExpr, WhenValue};
    let tier_supervised = PolicyTier {
        name: "supervised".to_string(),
        promote_when: Some(WhenExpr::Comparison(WhenComparison {
            field: metric.to_string(),
            op: CompareOp::Gt,
            value: WhenValue::Number(threshold.to_string()),
        })),
        span: Span { start: 0, end: 1 },
    };
    let tier_autonomous = PolicyTier {
        name: "autonomous".to_string(),
        promote_when: None,
        span: Span { start: 0, end: 1 },
    };
    let def = PolicyDef {
        tiers: vec![tier_supervised, tier_autonomous],
        span: Span { start: 0, end: 1 },
    };
    PolicyEngine::from_def(&def)
}

#[tokio::test]
async fn engine_with_policy_starts_at_first_tier() {
    let provider = MockProvider::new();
    provider.push_response(simple_response("done"));
    let executor = MockExecutor::new();
    let agent = make_agent(vec![], vec![], None);
    let registry = ToolRegistry::from_agent(&agent);
    let policy = make_policy_engine_with_threshold("tokens", 999_999.0);

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig::default(),
    )
    .with_policy(policy);
    let result = engine.run("hi").await.unwrap();
    assert_eq!(result.response, "done");
    // No promotion at low token counts — no PolicyPromotion event
    assert!(
        !result
            .trace
            .events
            .iter()
            .any(|e| matches!(e, RunEvent::PolicyPromotion { .. }))
    );
}

#[tokio::test]
async fn engine_with_policy_emits_promotion_event_when_threshold_met() {
    let provider = MockProvider::new();
    // One turn, costs ~150 tokens (100 input + 50 output per simple_response)
    provider.push_response(simple_response("done"));
    let executor = MockExecutor::new();
    let agent = make_agent(vec![], vec![], None);
    let registry = ToolRegistry::from_agent(&agent);
    // Threshold of 1 token — will promote immediately after first LLM call
    let policy = make_policy_engine_with_threshold("tokens", 1.0);

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig::default(),
    )
    .with_policy(policy);
    let result = engine.run("hi").await.unwrap();
    assert_eq!(result.response, "done");
    let promoted = result.trace.events.iter().any(|e| {
        matches!(e, RunEvent::PolicyPromotion { from_tier, to_tier }
            if from_tier == "supervised" && to_tier == "autonomous")
    });
    assert!(promoted, "expected PolicyPromotion event");
}

#[tokio::test]
async fn engine_without_policy_has_no_promotion_events() {
    let provider = MockProvider::new();
    provider.push_response(simple_response("done"));
    let executor = MockExecutor::new();
    let agent = make_agent(vec![], vec![], None);
    let registry = ToolRegistry::from_agent(&agent);

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig::default(),
    );
    let result = engine.run("hi").await.unwrap();
    assert!(
        !result
            .trace
            .events
            .iter()
            .any(|e| matches!(e, RunEvent::PolicyPromotion { .. }))
    );
}

// --- #304 OtelMode Tests ---

#[tokio::test]
async fn engine_default_otel_mode_is_none() {
    let provider = MockProvider::new();
    provider.push_response(simple_response("done"));
    let executor = MockExecutor::new();
    let agent = make_agent(vec![], vec![], None);
    let registry = ToolRegistry::from_agent(&agent);

    // Without with_otel_mode(), the engine should not panic or error
    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig::default(),
    );
    let result = engine.run("hi").await.unwrap();
    assert_eq!(result.response, "done");
}

#[tokio::test]
async fn engine_with_otel_mode_stdout_runs_successfully() {
    let provider = MockProvider::new();
    provider.push_response(simple_response("done"));
    let executor = MockExecutor::new();
    let agent = make_agent(vec![], vec![], None);
    let registry = ToolRegistry::from_agent(&agent);

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig::default(),
    )
    .with_otel_mode(OtelMode::StdoutOnComplete { metrics: vec![] });
    let result = engine.run("hi").await.unwrap();
    assert_eq!(result.response, "done");
}

#[tokio::test]
async fn engine_with_otel_mode_none_runs_successfully() {
    let provider = MockProvider::new();
    provider.push_response(simple_response("done"));
    let executor = MockExecutor::new();
    let agent = make_agent(vec![], vec![], None);
    let registry = ToolRegistry::from_agent(&agent);

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig::default(),
    )
    .with_otel_mode(OtelMode::None);
    let result = engine.run("hi").await.unwrap();
    assert_eq!(result.response, "done");
}

// --- #306 SecretsDef Tests ---

#[tokio::test]
async fn engine_with_secrets_runs_successfully() {
    let provider = MockProvider::new();
    provider.push_response(simple_response("done"));
    let executor = MockExecutor::new();
    let agent = make_agent(vec![], vec![], None);
    let registry = ToolRegistry::from_agent(&agent);

    let mut secrets = HashMap::new();
    secrets.insert("api_key".to_string(), "secret-value-123".to_string());

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig::default(),
    )
    .with_secrets(secrets);
    let result = engine.run("hi").await.unwrap();
    assert_eq!(result.response, "done");
}

#[tokio::test]
async fn engine_secrets_not_leaked_in_trace() {
    let provider = MockProvider::new();
    provider.push_response(simple_response("done"));
    let executor = MockExecutor::new();
    let agent = make_agent(vec![], vec![], None);
    let registry = ToolRegistry::from_agent(&agent);

    let mut secrets = HashMap::new();
    secrets.insert("api_key".to_string(), "super-secret-value".to_string());

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig::default(),
    )
    .with_secrets(secrets);
    let result = engine.run("hi").await.unwrap();

    // Secrets are stored on the engine but never forwarded into RunEvent payloads.
    // This test is a guardrail: if a future change inadvertently pipes secret values
    // into trace events (e.g. as tool arguments or LLM messages), this will catch it.
    let trace_json = serde_json::to_string(&result.trace).unwrap();
    assert!(
        !trace_json.contains("super-secret-value"),
        "secret value leaked into trace"
    );
}

#[tokio::test]
async fn engine_without_secrets_runs_successfully() {
    let provider = MockProvider::new();
    provider.push_response(simple_response("done"));
    let executor = MockExecutor::new();
    let agent = make_agent(vec![], vec![], None);
    let registry = ToolRegistry::from_agent(&agent);

    // No with_secrets() call — default empty map.
    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig::default(),
    );
    let result = engine.run("hi").await.unwrap();
    assert_eq!(result.response, "done");
}

// --- #318: event_matches_metrics unit tests ---

#[test]
fn event_matches_cost_metric() {
    let llm = RunEvent::LlmCall {
        model: "gpt-4o".into(),
        input_tokens: 10,
        output_tokens: 5,
        cost_cents: 1,
    };
    let budget = RunEvent::BudgetUpdate {
        spent_cents: 1,
        limit_cents: 100,
    };
    let tool = RunEvent::ToolCallAttempt {
        tool: crate::runtime::ToolCall {
            namespace: "a".into(),
            action: "b".into(),
            arguments: serde_json::json!({}),
        },
        allowed: true,
        reason: None,
    };
    assert!(event_matches_metrics(&llm, &["cost".to_string()]));
    assert!(event_matches_metrics(&budget, &["cost".to_string()]));
    assert!(!event_matches_metrics(&tool, &["cost".to_string()]));
}

#[test]
fn event_matches_tool_calls_metric() {
    let attempt = RunEvent::ToolCallAttempt {
        tool: crate::runtime::ToolCall {
            namespace: "a".into(),
            action: "b".into(),
            arguments: serde_json::json!({}),
        },
        allowed: true,
        reason: None,
    };
    let result_ev = RunEvent::ToolCallResult {
        tool: crate::runtime::ToolCall {
            namespace: "a".into(),
            action: "b".into(),
            arguments: serde_json::json!({}),
        },
        result: crate::runtime::ToolResult {
            success: true,
            output: "ok".into(),
        },
    };
    let budget = RunEvent::BudgetUpdate {
        spent_cents: 1,
        limit_cents: 100,
    };
    assert!(event_matches_metrics(&attempt, &["tool_calls".to_string()]));
    assert!(event_matches_metrics(
        &result_ev,
        &["tool_calls".to_string()]
    ));
    assert!(!event_matches_metrics(&budget, &["tool_calls".to_string()]));
}

#[test]
fn event_matches_latency_metric() {
    let llm = RunEvent::LlmCall {
        model: "gpt-4o".into(),
        input_tokens: 10,
        output_tokens: 5,
        cost_cents: 1,
    };
    let budget = RunEvent::BudgetUpdate {
        spent_cents: 1,
        limit_cents: 100,
    };
    assert!(event_matches_metrics(&llm, &["latency".to_string()]));
    assert!(!event_matches_metrics(&budget, &["latency".to_string()]));
}

#[test]
fn event_matches_guardrails_metric() {
    let guardrail = RunEvent::GuardrailTriggered {
        rule: "no_pii".into(),
        action: "block".into(),
        blocked: true,
    };
    let llm = RunEvent::LlmCall {
        model: "gpt-4o".into(),
        input_tokens: 10,
        output_tokens: 5,
        cost_cents: 1,
    };
    assert!(event_matches_metrics(
        &guardrail,
        &["guardrails".to_string()]
    ));
    assert!(!event_matches_metrics(&llm, &["guardrails".to_string()]));
}

#[test]
fn event_matches_unknown_metric_returns_false() {
    let llm = RunEvent::LlmCall {
        model: "gpt-4o".into(),
        input_tokens: 10,
        output_tokens: 5,
        cost_cents: 1,
    };
    assert!(!event_matches_metrics(
        &llm,
        &["not_a_real_metric".to_string()]
    ));
    assert!(!event_matches_metrics(&llm, &[]));
}

#[test]
// Covers variants that are intentionally NOT mapped to any metric category.
// These events must return false for every known metric name to prevent
// internal runtime events from leaking into filtered OTEL exports.
// Note: RunComplete IS mapped to "cost" and is excluded from this test —
// it is covered by `run_complete_matches_cost_metric` below.
fn unmapped_variants_return_false_for_all_known_metrics() {
    let all_metrics = ["cost", "tool_calls", "latency", "guardrails"];
    let cb = RunEvent::CircuitBreakerTripped {
        name: "cb".to_string(),
        failures: 3,
        threshold: 3,
    };
    for metric in &all_metrics {
        assert!(
            !event_matches_metrics(&cb, &[metric.to_string()]),
            "CircuitBreakerTripped should not match metric '{metric}'"
        );
    }
}

// #330: RunComplete carries total_cost_cents + total_tokens and must match "cost".
#[test]
fn run_complete_matches_cost_metric() {
    let ev = RunEvent::RunComplete {
        total_cost_cents: 42,
        total_tokens: 1000,
    };
    assert!(event_matches_metrics(&ev, &["cost".to_string()]));
    // Does NOT match non-cost metrics.
    assert!(!event_matches_metrics(&ev, &["tool_calls".to_string()]));
    assert!(!event_matches_metrics(&ev, &["latency".to_string()]));
    assert!(!event_matches_metrics(&ev, &["guardrails".to_string()]));
}

// #335: Verify secrets injected via with_secrets() reach executor.execute() ctx.
// Uses a SecretCapturingExecutor that records the last secrets map seen.
struct SecretCapturingExecutor {
    captured: Mutex<Option<crate::runtime::executor::Secrets>>,
    response: String,
}

impl SecretCapturingExecutor {
    fn new(response: impl Into<String>) -> Self {
        Self {
            captured: Mutex::new(None),
            response: response.into(),
        }
    }

    fn captured_secrets(&self) -> Option<crate::runtime::executor::Secrets> {
        self.captured.lock().expect("lock").clone()
    }
}

#[async_trait::async_trait]
impl crate::runtime::executor::ToolExecutor for SecretCapturingExecutor {
    async fn execute(
        &self,
        ctx: &crate::runtime::executor::ToolCallContext<'_>,
    ) -> Result<crate::runtime::executor::ToolOutput, crate::runtime::executor::ExecutorError> {
        *self.captured.lock().expect("lock") = Some(ctx.secrets.clone());
        Ok(crate::runtime::executor::ToolOutput {
            success: true,
            output: self.response.clone(),
        })
    }
}

#[tokio::test]
async fn engine_with_secrets_forwards_them_to_executor_context() {
    let provider = MockProvider::new();
    // First response triggers a tool call; second response finishes the run.
    provider.push_response(tool_call_response("zendesk.read_ticket", json!({"id": 1})));
    provider.push_response(simple_response("done"));

    let agent = make_agent(vec![cap("zendesk", "read_ticket")], vec![], None);
    let registry = crate::runtime::permissions::ToolRegistry::from_agent(&agent);

    let executor = SecretCapturingExecutor::new("ticket data");
    let mut secrets = HashMap::new();
    secrets.insert("API_TOKEN".to_string(), "s3cr3t".to_string());

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig::default(),
    )
    .with_secrets(secrets.clone());

    engine.run("hello").await.expect("run should succeed");

    let captured = executor
        .captured_secrets()
        .expect("executor should have been called");
    assert_eq!(
        captured.get("API_TOKEN").map(String::as_str),
        Some("s3cr3t")
    );
}

// ── Monetary cap tests (#349) ─────────────────────────────────────────────

fn capped_cap(ns: &str, action: &str, max_cents: u64) -> Capability {
    use crate::ast::Constraint;
    Capability {
        namespace: ns.to_string(),
        action: action.to_string(),
        constraint: Some(Constraint::MonetaryCap {
            amount: max_cents,
            currency: "USD".to_string(),
        }),
        span: Span { start: 0, end: 1 },
    }
}

// When cumulative LLM cost attributed to a capped tool reaches the cap,
// further calls to that tool must be denied.
#[tokio::test]
async fn capped_tool_denied_after_cap_exceeded() {
    // Cap api.call at 1 cent.
    // Mock LLM cost: 100 input + 50 output = 150 tokens at gpt-4o rates.
    // gpt-4o pricing: 250¢/M input + 1000¢/M output.
    // Cost = (100 × 250 + 50 × 1000) / 1_000_000 = 0.075¢ → div_ceil → 1¢.
    // After Turn 1: per_tool_spent["api.call"] = 1¢ = cap → Turn 2 call is denied.
    let agent = make_agent(vec![capped_cap("api", "call", 1)], vec![], None);
    let registry = ToolRegistry::from_agent(&agent);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // Turn 1: tool call allowed (spent 0 < cap 1), cost attributed → spent = 1.
    // Turn 2: tool call denied (spent 1 >= cap 1), LLM sees denial message.
    // Turn 3: LLM gives final answer.
    provider.push_response(tool_call_response("api.call", json!({})));
    provider.push_response(tool_call_response("api.call", json!({})));
    provider.push_response(simple_response("done"));

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig::default(),
    );
    let result = engine.run("go").await.expect("should complete");

    let allowed_count = result
        .trace
        .events
        .iter()
        .filter(|e| matches!(e, RunEvent::ToolCallAttempt { allowed: true, .. }))
        .count();
    let denied_count = result
        .trace
        .events
        .iter()
        .filter(|e| matches!(e, RunEvent::ToolCallAttempt { allowed: false, .. }))
        .count();

    assert_eq!(
        allowed_count, 1,
        "exactly one call should be allowed (within cap)"
    );
    assert_eq!(
        denied_count, 1,
        "exactly one call should be denied (cap reached)"
    );
}

// A tool with a generous cap should be allowed as long as cost stays under it.
#[tokio::test]
async fn capped_tool_allowed_within_cap() {
    // Cap api.call at 9999 cents — effectively unlimited for this test.
    let agent = make_agent(vec![capped_cap("api", "call", 9_999)], vec![], None);
    let registry = ToolRegistry::from_agent(&agent);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    provider.push_response(tool_call_response("api.call", json!({})));
    provider.push_response(simple_response("done"));

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig::default(),
    );
    let result = engine.run("go").await.expect("should complete");

    let denied_count = result
        .trace
        .events
        .iter()
        .filter(|e| matches!(e, RunEvent::ToolCallAttempt { allowed: false, .. }))
        .count();
    assert_eq!(
        denied_count, 0,
        "tool within cap should not be denied; trace: {:?}",
        result.trace.events
    );
}

// #355: stage_timeout_secs must cause engine.run() to return RunError::Timeout
// when the provider does not respond within the configured window.
// The partial trace in the error must contain a StageTimeout event.
#[tokio::test(start_paused = true)]
async fn stage_timeout_fires_when_provider_hangs() {
    let agent = make_agent(vec![], vec![], None);
    let registry = ToolRegistry::from_agent(&agent);
    let executor = MockExecutor::new();
    let provider = HangingProvider;

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig {
            stage_timeout_secs: Some(5),
            ..RunConfig::default()
        },
    );

    // start_paused = true: tokio auto-advances mock time when all tasks are
    // waiting on timers. The 5-second timeout will fire automatically.
    let result = engine.run("hello").await;

    // Destructure directly — this verifies the correct variant and lets us
    // inspect the partial trace in a single binding.
    let Err(RunError::Timeout { partial_trace }) = result else {
        panic!("expected RunError::Timeout, got a different result");
    };
    let has_timeout_event = partial_trace.events.iter().any(|e| {
        matches!(
            e,
            RunEvent::StageTimeout {
                turn: 0,
                timeout_secs: 5
            }
        )
    });
    assert!(
        has_timeout_event,
        "partial_trace must contain a StageTimeout event; got: {:?}",
        partial_trace.events
    );
    // The StageTimeout event must be last — the doc comment on RunError::Timeout
    // establishes this ordering contract.
    assert!(
        matches!(
            partial_trace.events.last(),
            Some(RunEvent::StageTimeout { .. })
        ),
        "StageTimeout must be the last event in the partial trace; got: {:?}",
        partial_trace.events
    );
}

// #355: when stage_timeout_secs is None (the default), existing runs complete
// normally — no timeout is applied and no regression is introduced.
#[tokio::test]
async fn no_timeout_when_stage_timeout_secs_is_none() {
    let agent = make_agent(vec![], vec![], None);
    let registry = ToolRegistry::from_agent(&agent);
    let executor = MockExecutor::new();
    let provider = MockProvider::new();
    provider.push_response(simple_response("done"));

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig {
            stage_timeout_secs: None,
            ..RunConfig::default()
        },
    );

    let result = engine.run("hello").await;
    assert!(
        result.is_ok(),
        "run without timeout should succeed: {:?}",
        result
    );
}

// #355: stage_timeout_secs = 0 is treated as no timeout (same as None).
// #391: stage_timeout_secs=0 is rejected early with ConfigError — a zero-second
// timeout would fire before any I/O, giving a confusing failure.
#[tokio::test]
async fn zero_stage_timeout_returns_config_error() {
    let agent = make_agent(vec![], vec![], None);
    let registry = ToolRegistry::from_agent(&agent);
    let executor = MockExecutor::new();
    let provider = MockProvider::new();
    provider.push_response(simple_response("done"));

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig {
            stage_timeout_secs: Some(0),
            ..RunConfig::default()
        },
    );

    let err = engine.run("hello").await.unwrap_err();
    assert!(
        matches!(err, RunError::ConfigError),
        "stage_timeout_secs=0 must return ConfigError; got: {err:?}"
    );
}

// #355: a timeout counts as a provider failure for circuit-breaker purposes.
// The partial trace's last event must be StageTimeout, and the circuit breaker
// must open after a single timeout when threshold = 1.
#[tokio::test(start_paused = true)]
async fn stage_timeout_records_circuit_breaker_failure() {
    use crate::runtime::circuit_breaker::CircuitBreaker;

    let agent = make_agent(vec![], vec![], None);
    let registry = ToolRegistry::from_agent(&agent);
    let executor = MockExecutor::new();
    let provider = HangingProvider;

    // Circuit breaker with threshold = 1: a single failure opens it.
    let cb = CircuitBreaker::from_def(&crate::ast::CircuitBreakerDef {
        name: "test-cb".to_string(),
        failure_threshold: 1,
        window_minutes: 1,
        half_open_after_minutes: 1,
        span: crate::ast::Span { start: 0, end: 0 },
    });

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig {
            stage_timeout_secs: Some(1),
            ..RunConfig::default()
        },
    )
    .with_circuit_breaker(cb);

    // First run: timeout fires, circuit breaker records one failure.
    let result = engine.run("hello").await;
    assert!(
        matches!(result, Err(RunError::Timeout { .. })),
        "first run should time out"
    );
    // Partial trace must contain a StageTimeout event — same contract as the
    // non-CB timeout path.
    if let Err(RunError::Timeout { partial_trace }) = &result {
        assert!(
            partial_trace
                .events
                .iter()
                .any(|e| matches!(e, RunEvent::StageTimeout { .. })),
            "partial trace must contain StageTimeout; got: {:?}",
            partial_trace.events
        );
    }

    // Second run: circuit breaker is now open after the failure above.
    let result2 = engine.run("hello").await;
    assert!(
        matches!(result2, Err(RunError::CircuitBreakerOpen { .. })),
        "second run should be blocked by open circuit breaker; got: {:?}",
        result2
    );
}

// #424: timeout on turn > 0 must carry prior events in partial_trace.
// Turn 0 succeeds (tool call), turn 1 hangs — partial trace must contain
// the turn 0 LlmCall event AND the final StageTimeout { turn: 1 } event.
#[tokio::test(start_paused = true)]
async fn stage_timeout_on_turn_1_includes_prior_events_in_partial_trace() {
    let agent = make_agent(vec![cap("test", "noop")], vec![], None);
    let registry = ToolRegistry::from_agent(&agent);
    let executor = MockExecutor::new();
    executor.on_call("test", "noop", "ok");

    let provider = HangAfterFirstProvider::new();

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![ToolDef {
            name: "test.noop".to_string(),
            description: "no-op tool".to_string(),
            parameters: serde_json::json!({}),
        }],
        RunConfig {
            stage_timeout_secs: Some(5),
            ..RunConfig::default()
        },
    );

    let result = engine.run("hello").await;
    let Err(RunError::Timeout { partial_trace }) = result else {
        panic!("expected RunError::Timeout; got: {:?}", result);
    };

    // Must contain turn 0's LlmCall (from the successful first provider call).
    assert!(
        partial_trace
            .events
            .iter()
            .any(|e| matches!(e, RunEvent::LlmCall { .. })),
        "partial_trace must include turn 0 LlmCall; got: {:?}",
        partial_trace.events
    );

    // StageTimeout must be the last event, on turn 1.
    assert!(
        matches!(
            partial_trace.events.last(),
            Some(RunEvent::StageTimeout { turn: 1, .. })
        ),
        "last event must be StageTimeout {{ turn: 1, .. }}; got: {:?}",
        partial_trace.events
    );
}

// ── #390: BudgetUpdate on Exceeded path must report tracker-derived spent_cents ──

#[tokio::test]
async fn budget_update_exceeded_reports_correct_spent_cents() {
    // Budget: 1 cent. The LLM response will cost much more.
    let agent = make_agent(vec![], vec![], Some(1));
    let registry = ToolRegistry::from_agent(&agent);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    provider.push_response(ChatResponse {
        content: String::new(),
        tool_calls: vec![],
        usage: Usage {
            input_tokens: 100_000,
            output_tokens: 50_000,
        },
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

    let err = engine.run("hi").await.unwrap_err();
    let RunError::BudgetExceeded { partial_trace } = err else {
        panic!("expected BudgetExceeded, got: {err:?}");
    };
    // Verify the BudgetUpdate event in the partial trace carries the correct fields.
    // spent_cents: 100_000 input + 50_000 output for gpt-4o = (100_000 * 250 + 50_000 * 1_000) / 1_000_000
    //            = (25_000_000 + 50_000_000) / 1_000_000 = 75 cents (rounded up from fractional).
    // limit_cents: 1 (the configured budget).
    let budget_event = partial_trace
        .events
        .iter()
        .find(|e| matches!(e, crate::runtime::RunEvent::BudgetUpdate { .. }))
        .expect("BudgetUpdate event must be in partial trace");
    let crate::runtime::RunEvent::BudgetUpdate {
        spent_cents,
        limit_cents,
    } = budget_event
    else {
        unreachable!()
    };
    assert!(
        *spent_cents > 1,
        "spent_cents must exceed the 1-cent limit; got {spent_cents}"
    );
    assert_eq!(
        *limit_cents, 1,
        "limit_cents must match the configured budget"
    );
}

// ── #389: CircuitBreakerTripped event must carry real failure count + threshold ──

#[tokio::test]
async fn circuit_breaker_tripped_event_has_real_failures_and_threshold() {
    use crate::runtime::circuit_breaker::CircuitBreaker;

    let agent = make_agent(vec![], vec![], None);
    let registry = ToolRegistry::from_agent(&agent);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // Push a simple response (won't be reached because CB is pre-tripped)
    provider.push_response(simple_response("hi"));

    // Build a circuit breaker with threshold=2, then trip it by recording 2 failures.
    let mut cb = CircuitBreaker::new("test_cb", 2, 5, 1);
    cb.record_failure();
    cb.record_failure(); // trips the breaker

    let engine = AgentEngine::new(
        &provider,
        &executor,
        &registry,
        vec![],
        RunConfig::default(),
    )
    .with_circuit_breaker(cb);

    let result = engine.run("hello").await;
    let RunError::CircuitBreakerOpen { partial_trace } = result.unwrap_err() else {
        panic!("expected CircuitBreakerOpen");
    };
    // Verify the CircuitBreakerTripped event in the partial trace has the real values.
    let tripped = partial_trace
        .events
        .iter()
        .find(|e| matches!(e, crate::runtime::RunEvent::CircuitBreakerTripped { .. }))
        .expect("CircuitBreakerTripped event must be in partial trace");
    let crate::runtime::RunEvent::CircuitBreakerTripped {
        name,
        failures,
        threshold,
    } = tripped
    else {
        unreachable!()
    };
    assert_eq!(name, "test_cb");
    assert_eq!(
        *failures, 2,
        "failures must equal the number of recorded failures"
    );
    assert_eq!(
        *threshold, 2,
        "threshold must equal the configured threshold"
    );
}

// ── #407: RunError must implement std::error::Error ──

#[test]
fn run_error_implements_std_error() {
    // Verify RunError can be used as dyn Error (compile-time check via trait object).
    let err: &dyn std::error::Error = &RunError::BudgetExceeded {
        partial_trace: crate::runtime::RunTrace::from_events(vec![]),
    };
    assert_eq!(err.to_string(), "budget exceeded");
}

#[test]
fn run_error_timeout_implements_std_error() {
    use crate::runtime::RunTrace;
    let err: &dyn std::error::Error = &RunError::Timeout {
        partial_trace: RunTrace::from_events(vec![]),
    };
    assert_eq!(err.to_string(), "provider timed out");
}

// --- #490: GuardrailBlocked must carry partial_trace from engine ---

/// #490: When AgentEngine::run() returns RunError::GuardrailBlocked, the
/// partial_trace must contain the GuardrailTriggered event that fired before
/// the block. This allows callers to inspect which rule triggered the block
/// and what content caused it.
#[tokio::test]
async fn guardrail_blocked_carries_partial_trace_with_triggered_event() {
    use crate::ast::{GuardrailRule, GuardrailSection, GuardrailsDef, Span};
    use crate::runtime::guardrails::GuardrailEngine;
    use crate::runtime::RunEvent;

    // Build a guardrail engine that blocks PII (emails).
    let def = GuardrailsDef {
        sections: vec![GuardrailSection {
            name: "output".to_string(),
            rules: vec![GuardrailRule {
                key: "pii_detection".to_string(),
                value: "block".to_string(),
                span: Span { start: 0, end: 0 },
            }],
            span: Span { start: 0, end: 0 },
        }],
        span: Span { start: 0, end: 0 },
    };
    let guardrails = GuardrailEngine::from_def(&def);

    let agent = make_agent(vec![], vec![], None);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // The LLM response contains PII — this will trigger the block.
    provider.push_response(simple_response(
        "Contact me at user@example.com for support.",
    ));

    let registry = crate::runtime::permissions::ToolRegistry::from_agent(&agent);
    let engine = AgentEngine::new(&provider, &executor, &registry, vec![], RunConfig::default())
        .with_guardrails(guardrails);

    let err = engine.run("hello").await.unwrap_err();

    let RunError::GuardrailBlocked { partial_trace } = err else {
        panic!("expected RunError::GuardrailBlocked, got another error");
    };

    // partial_trace must contain the GuardrailTriggered event.
    let has_triggered = partial_trace.events.iter().any(|e| {
        matches!(
            e,
            RunEvent::GuardrailTriggered {
                rule,
                blocked,
                ..
            }
            if rule == "pii_detection" && *blocked
        )
    });
    assert!(
        has_triggered,
        "partial_trace must contain GuardrailTriggered(pii_detection, blocked=true); \
         got events: {:?}",
        partial_trace.events
    );
}
