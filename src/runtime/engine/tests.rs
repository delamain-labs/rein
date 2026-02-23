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
#[tokio::test(start_paused = true)]
async fn stage_timeout_fires_when_provider_hangs() {
    use crate::runtime::provider::{ChatResponse, Message, ProviderError, ToolDef};

    struct HangingProvider;
    #[async_trait::async_trait]
    impl crate::runtime::provider::Provider for HangingProvider {
        fn name(&self) -> &'static str { "hanging" }
        async fn chat(
            &self,
            _messages: &[Message],
            _tools: &[ToolDef],
        ) -> Result<ChatResponse, ProviderError> {
            // Never returns — tokio mock clock will advance past the timeout.
            std::future::pending().await
        }
    }

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
    assert!(
        matches!(result, Err(RunError::Timeout)),
        "expected RunError::Timeout, got: {:?}",
        result
    );
}
