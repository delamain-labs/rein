use super::*;
use crate::ast::{ExecutionMode, RouteRule, Span, Stage};
use crate::runtime::executor::MockExecutor;
use crate::runtime::provider::{ChatResponse, MockProvider, Usage};

fn simple_response(content: &str) -> ChatResponse {
    ChatResponse {
        content: content.to_string(),
        tool_calls: vec![],
        usage: Usage { input_tokens: 100, output_tokens: 50 },
        model: "gpt-4o".to_string(),
    }
}

fn parse_file(src: &str) -> ReinFile {
    crate::parser::parse(src).expect("parse should succeed")
}

fn make_workflow(name: &str, trigger: &str, stage_agents: &[&str]) -> WorkflowDef {
    WorkflowDef {
        name: name.to_string(),
        trigger: trigger.to_string(),
        stages: stage_agents
            .iter()
            .map(|a| Stage {
                name: (*a).to_string(),
                agent: (*a).to_string(),
                route: RouteRule::Next,
                span: Span::new(0, 1),
            })
            .collect(),
        mode: ExecutionMode::Sequential,
        span: Span::new(0, 1),
    }
}

#[tokio::test]
async fn single_stage_workflow() {
    let file = parse_file(r#"
        agent triage { model: openai can [ zendesk.read_ticket ] }
    "#);
    let workflow = make_workflow("pipe", "ticket", &["triage"]);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    provider.push_response(simple_response("Triaged: low priority"));

    let result = run_sequential(&workflow, &file, &provider, &executor, &[], &RunConfig::default())
        .await
        .expect("should succeed");

    assert_eq!(result.stage_results.len(), 1);
    assert_eq!(result.final_output, "Triaged: low priority");
    assert!(result.total_tokens > 0);
}

#[tokio::test]
async fn two_stage_pipeline_passes_output() {
    let file = parse_file(r#"
        agent triage { model: openai }
        agent responder { model: openai }
    "#);
    let workflow = make_workflow("pipe", "ticket", &["triage", "responder"]);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // Stage 1: triage
    provider.push_response(simple_response("Priority: high. Issue: billing error."));
    // Stage 2: responder (receives triage output as input)
    provider.push_response(simple_response("Dear customer, we've fixed your billing issue."));

    let result = run_sequential(&workflow, &file, &provider, &executor, &[], &RunConfig::default())
        .await
        .expect("should succeed");

    assert_eq!(result.stage_results.len(), 2);
    assert_eq!(result.stage_results[0].output, "Priority: high. Issue: billing error.");
    assert_eq!(result.final_output, "Dear customer, we've fixed your billing issue.");
    assert_eq!(result.total_cost_cents, result.stage_results[0].cost_cents + result.stage_results[1].cost_cents);
}

#[tokio::test]
async fn three_stage_pipeline() {
    let file = parse_file(r#"
        agent a { model: openai }
        agent b { model: openai }
        agent c { model: openai }
    "#);
    let workflow = make_workflow("pipe", "event", &["a", "b", "c"]);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    provider.push_response(simple_response("output_a"));
    provider.push_response(simple_response("output_b"));
    provider.push_response(simple_response("output_c"));

    let result = run_sequential(&workflow, &file, &provider, &executor, &[], &RunConfig::default())
        .await
        .expect("should succeed");

    assert_eq!(result.stage_results.len(), 3);
    assert_eq!(result.final_output, "output_c");
}

#[tokio::test]
async fn unknown_agent_returns_error() {
    let file = parse_file("agent a { model: openai }");
    let workflow = make_workflow("pipe", "event", &["a", "nonexistent"]);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    provider.push_response(simple_response("ok"));

    let err = run_sequential(&workflow, &file, &provider, &executor, &[], &RunConfig::default())
        .await
        .unwrap_err();

    assert!(err.to_string().contains("nonexistent"), "err: {err}");
}

#[tokio::test]
async fn stage_failure_returns_error() {
    let file = parse_file("agent a { model: openai }");
    let workflow = make_workflow("pipe", "event", &["a"]);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    provider.push_error("provider down");

    let err = run_sequential(&workflow, &file, &provider, &executor, &[], &RunConfig::default())
        .await
        .unwrap_err();

    assert!(matches!(err, WorkflowError::StageFailed { .. }));
}

// ── Parallel workflow tests ──────────────────────────────────────────────

#[tokio::test]
async fn parallel_workflow_runs_all_stages() {
    let file = parse_file(r#"
        agent a { model: openai }
        agent b { model: openai }
    "#);
    let mut workflow = make_workflow("pipe", "event", &["a", "b"]);
    workflow.mode = ExecutionMode::Parallel;

    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    provider.push_response(simple_response("output_a"));
    provider.push_response(simple_response("output_b"));

    let result = run_parallel(&workflow, &file, &provider, &executor, &[], &RunConfig::default())
        .await
        .expect("should succeed");

    assert_eq!(result.stage_results.len(), 2);
    assert_eq!(result.stage_results[0].output, "output_a");
    assert_eq!(result.stage_results[1].output, "output_b");
    assert!(result.final_output.contains("output_a"));
    assert!(result.final_output.contains("output_b"));
}

#[tokio::test]
async fn parallel_unknown_agent_errors() {
    let file = parse_file("agent a { model: openai }");
    let mut workflow = make_workflow("pipe", "event", &["a", "missing"]);
    workflow.mode = ExecutionMode::Parallel;

    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // Queue response for agent "a" so it doesn't fail first
    provider.push_response(simple_response("ok"));

    let err = run_parallel(&workflow, &file, &provider, &executor, &[], &RunConfig::default())
        .await
        .unwrap_err();

    assert!(err.to_string().contains("missing"), "err: {err}");
}

#[tokio::test]
async fn run_workflow_dispatches_by_mode() {
    let file = parse_file("agent a { model: openai }");
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // Sequential
    let mut seq = make_workflow("seq", "event", &["a"]);
    seq.mode = ExecutionMode::Sequential;
    provider.push_response(simple_response("sequential"));
    let r1 = run_workflow(&seq, &file, &provider, &executor, &[], &RunConfig::default())
        .await
        .expect("ok");
    assert_eq!(r1.final_output, "sequential");

    // Parallel
    let mut par = make_workflow("par", "event", &["a"]);
    par.mode = ExecutionMode::Parallel;
    provider.push_response(simple_response("parallel"));
    let r2 = run_workflow(&par, &file, &provider, &executor, &[], &RunConfig::default())
        .await
        .expect("ok");
    assert!(r2.final_output.contains("parallel"));
}
