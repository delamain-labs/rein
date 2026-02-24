/// Integration tests for the full agent runtime pipeline.
///
/// Tests the flow: parse .rein → build engine → run with mocks → verify trace.
use std::sync::Arc;

use serde_json::json;

use rein::runtime::approval::{AutoApproveHandler, AuditingApprovalHandler};
use rein::runtime::audit::{AuditKind, AuditLog};
use rein::runtime::engine::{AgentEngine, RunConfig};
use rein::runtime::executor::MockExecutor;
use rein::runtime::permissions::ToolRegistry;
use rein::runtime::provider::{ChatResponse, MockProvider, ToolCallRequest, ToolDef, Usage};
use rein::runtime::{RunError, RunEvent};

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

fn tool_call_response(name: &str, args: serde_json::Value) -> ChatResponse {
    ChatResponse {
        content: String::new(),
        tool_calls: vec![ToolCallRequest {
            id: "call_1".to_string(),
            name: name.to_string(),
            arguments: args,
        }],
        usage: Usage {
            input_tokens: 100,
            output_tokens: 50,
        },
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
        RunConfig {
            budget_cents: 100,
            ..RunConfig::default()
        },
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
    executor.on_call(
        "zendesk",
        "read_ticket",
        r#"{"id": 42, "subject": "Billing"}"#,
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
        .run("What's ticket 42?")
        .await
        .expect("should succeed");
    assert_eq!(result.response, "Ticket 42 is about billing.");

    // Verify tool was allowed in trace
    let allowed: Vec<_> = result
        .trace
        .events
        .iter()
        .filter(|e| matches!(e, RunEvent::ToolCallAttempt { allowed: true, .. }))
        .collect();
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

    let result = engine
        .run("Delete ticket 42")
        .await
        .expect("should succeed");
    assert_eq!(result.response, "I cannot delete tickets.");

    // Verify tool was denied in trace
    let denied: Vec<_> = result
        .trace
        .events
        .iter()
        .filter(|e| matches!(e, RunEvent::ToolCallAttempt { allowed: false, .. }))
        .collect();
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

    let err = engine.run("Hi").await.unwrap_err();
    assert!(matches!(err, RunError::BudgetExceeded { .. }));
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

// ── Workflow integration tests ───────────────────────────────────────────

use rein::ast::{ExecutionMode, RouteRule, Span, Stage, WorkflowDef};
use rein::runtime::workflow::{
    WorkflowContext, run_parallel, run_sequential, run_sequential_resumable,
};

fn make_workflow(name: &str, trigger: &str, agents: &[&str]) -> WorkflowDef {
    WorkflowDef {
        name: name.to_string(),
        trigger: trigger.to_string(),
        stages: agents
            .iter()
            .map(|a| Stage {
                name: (*a).to_string(),
                agent: (*a).to_string(),
                route: RouteRule::Next,
                span: Span::new(0, 1),
            })
            .collect(),
        steps: vec![],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: None,
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: Span::new(0, 1),
    }
}

#[tokio::test]
async fn integration_sequential_workflow() {
    let source = r#"
        agent classifier { model: openai can [ zendesk.classify ] }
        agent responder { model: openai can [ zendesk.reply_ticket ] }
    "#;
    let file = rein::parser::parse(source).expect("parse");
    let workflow = make_workflow("pipe", "ticket_123", &["classifier", "responder"]);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();
    let ctx = WorkflowContext {
        file: &file,
        provider: &provider,
        executor: &executor,
        tool_defs: &[],
        config: &RunConfig::default(),
        approval_handler: None,
        audit_log: None,
        workflow_name: None,
    };

    provider.push_response(simple_response("Category: billing. Priority: high."));
    provider.push_response(simple_response(
        "Dear customer, we've resolved your billing issue.",
    ));

    let result = run_sequential(&workflow, &ctx).await.expect("ok");

    assert_eq!(result.stage_results.len(), 2);
    assert_eq!(result.stage_results[0].agent_name, "classifier");
    assert_eq!(result.stage_results[1].agent_name, "responder");
    assert!(result.final_output.contains("billing issue"));

    // #388: event_timestamps_ms must be parallel to events — no silent drop of
    // timing data when events flow through WorkflowResult.
    assert_eq!(
        result.event_timestamps_ms.len(),
        result.events.len(),
        "event_timestamps_ms must have one entry per event"
    );
}

#[tokio::test]
async fn integration_parallel_workflow() {
    let source = r#"
        agent sentiment { model: openai }
        agent summary { model: openai }
    "#;
    let file = rein::parser::parse(source).expect("parse");
    let mut workflow = make_workflow("analyze", "document", &["sentiment", "summary"]);
    workflow.mode = ExecutionMode::Parallel;
    let provider = MockProvider::new();
    let executor = MockExecutor::new();
    let ctx = WorkflowContext {
        file: &file,
        provider: &provider,
        executor: &executor,
        tool_defs: &[],
        config: &RunConfig::default(),
        approval_handler: None,
        audit_log: None,
        workflow_name: None,
    };

    provider.push_response(simple_response("Sentiment: positive"));
    provider.push_response(simple_response("Summary: quarterly results are up"));

    let result = run_parallel(&workflow, &ctx).await.expect("ok");

    assert_eq!(result.stage_results.len(), 2);
    assert!(result.final_output.contains("Sentiment"));
    assert!(result.final_output.contains("Summary"));

    // #388: event_timestamps_ms must be parallel to events for parallel workflows too.
    assert_eq!(
        result.event_timestamps_ms.len(),
        result.events.len(),
        "event_timestamps_ms must have one entry per event"
    );
}

#[tokio::test]
async fn integration_resumable_workflow_timestamps_parallel() {
    let source = r#"
        agent classifier { model: openai can [ zendesk.classify ] }
        agent responder { model: openai can [ zendesk.reply_ticket ] }
    "#;
    let file = rein::parser::parse(source).expect("parse");
    let workflow = make_workflow("pipe", "ticket_123", &["classifier", "responder"]);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();
    let ctx = WorkflowContext {
        file: &file,
        provider: &provider,
        executor: &executor,
        tool_defs: &[],
        config: &RunConfig::default(),
        approval_handler: None,
        audit_log: None,
        workflow_name: None,
    };

    provider.push_response(simple_response("Category: billing."));
    provider.push_response(simple_response("Resolved your billing issue."));

    let state_path = std::env::temp_dir().join("rein_test_resumable_timestamps.json");
    let _ = std::fs::remove_file(&state_path); // start clean

    let result = run_sequential_resumable(&workflow, &ctx, &state_path)
        .await
        .expect("ok");

    assert_eq!(result.stage_results.len(), 2);

    // #388: event_timestamps_ms must be parallel to events for resumable workflows.
    assert_eq!(
        result.event_timestamps_ms.len(),
        result.events.len(),
        "event_timestamps_ms must have one entry per event in resumable path"
    );
}

// ── #438: --audit-log end-to-end integration test ───────────────────────────

/// #438: Running a workflow with `audit_log` set and an approval step must
/// produce a JSONL audit file containing at least one `ApprovalRequested` and
/// one `ApprovalResolved` entry, each with the correct `workflow` and `step`
/// fields.
#[tokio::test]
async fn audit_log_records_approval_requested_and_resolved() {
    use rein::ast::{
        ApprovalDef, ApprovalKind, ExecutionMode, RouteRule, Span, StepDef, WorkflowDef,
    };

    let source = r#"
        agent reviewer { model: openai }
    "#;
    let file = rein::parser::parse(source).expect("parse");

    // Build a step with an approval gate.
    let step = StepDef {
        name: "review_step".to_string(),
        agent: "reviewer".to_string(),
        goal: None,
        input: None,
        output_constraints: vec![],
        depends_on: vec![],
        when: None,
        on_failure: None,
        send_to: None,
        fallback: None,
        for_each: None,
        typed_input: None,
        typed_outputs: vec![],
        escalate: None,
        approval: Some(ApprovalDef {
            kind: ApprovalKind::Approve,
            channel: "slack".to_string(),
            destination: "#approvals".to_string(),
            timeout: None,
            mode: None,
            span: Span::new(0, 1),
        }),
        span: Span::new(0, 1),
    };

    let workflow = WorkflowDef {
        name: "approval_workflow".to_string(),
        trigger: "ticket".to_string(),
        stages: vec![],
        steps: vec![step],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: None,
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: Span::new(0, 1),
    };

    let provider = MockProvider::new();
    provider.push_response(simple_response("review result"));

    let executor = MockExecutor::new();

    // Point audit log at a temp file.
    let audit_path = std::env::temp_dir().join("rein_test_audit_438.jsonl");
    let _ = std::fs::remove_file(&audit_path);
    let audit_log = Arc::new(AuditLog::new(&audit_path).expect("audit log should be creatable"));

    // Pre-wrap with AuditingApprovalHandler — run_step no longer adds its own
    // wrapper around pre-injected handlers (#411).
    let inner: Arc<dyn rein::runtime::approval::ApprovalHandler> = Arc::new(AutoApproveHandler);
    let approval_handler: Arc<dyn rein::runtime::approval::ApprovalHandler> = Arc::new(
        AuditingApprovalHandler::with_context(
            inner,
            Arc::clone(&audit_log),
            Some("approval_workflow"),
            None::<&str>,
        ),
    );

    let ctx = rein::runtime::workflow::WorkflowContext {
        file: &file,
        provider: &provider,
        executor: &executor,
        tool_defs: &[],
        config: &RunConfig::default(),
        approval_handler: Some(approval_handler),
        audit_log: Some(Arc::clone(&audit_log)),
        workflow_name: Some("approval_workflow".to_string()),
    };

    rein::runtime::workflow::run_workflow(&workflow, &ctx)
        .await
        .expect("workflow should succeed");

    // Read audit JSONL entries.
    let entries = audit_log
        .read_all()
        .expect("audit log should be readable");

    // Assert at least one ApprovalRequested entry.
    let requested: Vec<_> = entries
        .iter()
        .filter(|e| e.kind == AuditKind::ApprovalRequested)
        .collect();
    assert!(
        !requested.is_empty(),
        "expected at least one ApprovalRequested in audit log; got entries: {entries:?}"
    );
    let req = &requested[0];
    assert_eq!(
        req.workflow.as_deref(),
        Some("approval_workflow"),
        "ApprovalRequested.workflow must be 'approval_workflow'"
    );
    assert_eq!(
        req.step.as_deref(),
        Some("review_step"),
        "ApprovalRequested.step must be 'review_step'"
    );

    // Assert at least one ApprovalResolved entry.
    let resolved: Vec<_> = entries
        .iter()
        .filter(|e| e.kind == AuditKind::ApprovalResolved)
        .collect();
    assert!(
        !resolved.is_empty(),
        "expected at least one ApprovalResolved in audit log; got entries: {entries:?}"
    );
    let res = &resolved[0];
    assert_eq!(
        res.workflow.as_deref(),
        Some("approval_workflow"),
        "ApprovalResolved.workflow must be 'approval_workflow'"
    );
    assert_eq!(
        res.step.as_deref(),
        Some("review_step"),
        "ApprovalResolved.step must be 'review_step'"
    );
}

/// #411: When a caller pre-wraps the approval_handler with AuditingApprovalHandler
/// before building WorkflowContext, run_step must NOT add a second audit wrapper.
/// Exactly ONE ApprovalRequested and ONE ApprovalResolved entry must appear.
#[tokio::test]
async fn no_double_audit_wrapping_when_handler_pre_wrapped() {
    use rein::ast::{
        ApprovalDef, ApprovalKind, ExecutionMode, RouteRule, Span, StepDef, WorkflowDef,
    };
    use rein::runtime::approval::ApprovalHandler;
    use rein::runtime::engine::RunConfig;

    let source = r#"agent reviewer { model: openai }"#;
    let file = rein::parser::parse(source).expect("parse");

    let step = StepDef {
        name: "review_step".to_string(),
        agent: "reviewer".to_string(),
        goal: None,
        input: None,
        output_constraints: vec![],
        depends_on: vec![],
        when: None,
        on_failure: None,
        send_to: None,
        fallback: None,
        for_each: None,
        typed_input: None,
        typed_outputs: vec![],
        escalate: None,
        approval: Some(ApprovalDef {
            kind: ApprovalKind::Approve,
            channel: "slack".to_string(),
            destination: "#approvals".to_string(),
            timeout: None,
            mode: None,
            span: Span::new(0, 1),
        }),
        span: Span::new(0, 1),
    };

    let workflow = WorkflowDef {
        name: "approval_wf".to_string(),
        trigger: "ticket".to_string(),
        stages: vec![],
        steps: vec![step],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: None,
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: Span::new(0, 1),
    };

    let provider = MockProvider::new();
    provider.push_response(simple_response("review result"));
    let executor = MockExecutor::new();

    let audit_path = std::env::temp_dir().join("rein_test_audit_411.jsonl");
    let _ = std::fs::remove_file(&audit_path);
    let audit_log = Arc::new(AuditLog::new(&audit_path).expect("audit log"));

    // Caller pre-wraps the handler — run_step must NOT add a second wrapper.
    let inner: Arc<dyn ApprovalHandler> = Arc::new(AutoApproveHandler);
    let pre_wrapped: Arc<dyn ApprovalHandler> = Arc::new(
        AuditingApprovalHandler::with_context(
            inner,
            Arc::clone(&audit_log),
            Some("approval_wf"),
            None::<&str>,
        ),
    );

    let ctx = rein::runtime::workflow::WorkflowContext {
        file: &file,
        provider: &provider,
        executor: &executor,
        tool_defs: &[],
        config: &RunConfig::default(),
        approval_handler: Some(pre_wrapped),
        audit_log: Some(Arc::clone(&audit_log)),
        workflow_name: Some("approval_wf".to_string()),
    };

    rein::runtime::workflow::run_workflow(&workflow, &ctx)
        .await
        .expect("workflow should succeed");

    let entries = audit_log.read_all().expect("should be readable");
    let req_count = entries
        .iter()
        .filter(|e| e.kind == AuditKind::ApprovalRequested)
        .count();
    assert_eq!(
        req_count, 1,
        "#411: must not double-wrap — expected exactly 1 ApprovalRequested entry, \
         got {req_count}; entries: {entries:?}"
    );
    let res_count = entries
        .iter()
        .filter(|e| e.kind == AuditKind::ApprovalResolved)
        .count();
    assert_eq!(
        res_count, 1,
        "#411: must not double-wrap — expected exactly 1 ApprovalResolved entry, \
         got {res_count}; entries: {entries:?}"
    );
}
