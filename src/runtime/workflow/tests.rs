use super::*;
use crate::ast::{ExecutionMode, RouteRule, Span, Stage, WorkflowDef};
use crate::runtime::executor::MockExecutor;
use crate::runtime::provider::{ChatResponse, MockProvider, Usage};

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
    let file = parse_file(
        r"
        agent triage { model: openai can [ zendesk.read_ticket ] }
    ",
    );
    let workflow = make_workflow("pipe", "ticket", &["triage"]);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    provider.push_response(simple_response("Triaged: low priority"));

    let result = run_sequential(
        &workflow,
        &file,
        &provider,
        &executor,
        &[],
        &RunConfig::default(),
    )
    .await
    .expect("should succeed");

    assert_eq!(result.stage_results.len(), 1);
    assert_eq!(result.final_output, "Triaged: low priority");
    assert!(result.total_tokens > 0);
}

#[tokio::test]
async fn two_stage_pipeline_passes_output() {
    let file = parse_file(
        r"
        agent triage { model: openai }
        agent responder { model: openai }
    ",
    );
    let workflow = make_workflow("pipe", "ticket", &["triage", "responder"]);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // Stage 1: triage
    provider.push_response(simple_response("Priority: high. Issue: billing error."));
    // Stage 2: responder (receives triage output as input)
    provider.push_response(simple_response(
        "Dear customer, we've fixed your billing issue.",
    ));

    let result = run_sequential(
        &workflow,
        &file,
        &provider,
        &executor,
        &[],
        &RunConfig::default(),
    )
    .await
    .expect("should succeed");

    assert_eq!(result.stage_results.len(), 2);
    assert_eq!(
        result.stage_results[0].output,
        "Priority: high. Issue: billing error."
    );
    assert_eq!(
        result.final_output,
        "Dear customer, we've fixed your billing issue."
    );
    assert_eq!(
        result.total_cost_cents,
        result.stage_results[0].cost_cents + result.stage_results[1].cost_cents
    );
}

#[tokio::test]
async fn three_stage_pipeline() {
    let file = parse_file(
        r"
        agent a { model: openai }
        agent b { model: openai }
        agent c { model: openai }
    ",
    );
    let workflow = make_workflow("pipe", "event", &["a", "b", "c"]);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    provider.push_response(simple_response("output_a"));
    provider.push_response(simple_response("output_b"));
    provider.push_response(simple_response("output_c"));

    let result = run_sequential(
        &workflow,
        &file,
        &provider,
        &executor,
        &[],
        &RunConfig::default(),
    )
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

    let err = run_sequential(
        &workflow,
        &file,
        &provider,
        &executor,
        &[],
        &RunConfig::default(),
    )
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

    let err = run_sequential(
        &workflow,
        &file,
        &provider,
        &executor,
        &[],
        &RunConfig::default(),
    )
    .await
    .unwrap_err();

    assert!(matches!(err, WorkflowError::StageFailed { .. }));
}

// ── Parallel workflow tests ──────────────────────────────────────────────

#[tokio::test]
async fn parallel_workflow_runs_all_stages() {
    let file = parse_file(
        r"
        agent a { model: openai }
        agent b { model: openai }
    ",
    );
    let mut workflow = make_workflow("pipe", "event", &["a", "b"]);
    workflow.mode = ExecutionMode::Parallel;

    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    provider.push_response(simple_response("output_a"));
    provider.push_response(simple_response("output_b"));

    let result = run_parallel(
        &workflow,
        &file,
        &provider,
        &executor,
        &[],
        &RunConfig::default(),
    )
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

    let err = run_parallel(
        &workflow,
        &file,
        &provider,
        &executor,
        &[],
        &RunConfig::default(),
    )
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
    let r1 = run_workflow(
        &seq,
        &file,
        &provider,
        &executor,
        &[],
        &RunConfig::default(),
    )
    .await
    .expect("ok");
    assert_eq!(r1.final_output, "sequential");

    // Parallel
    let mut par = make_workflow("par", "event", &["a"]);
    par.mode = ExecutionMode::Parallel;
    provider.push_response(simple_response("parallel"));
    let r2 = run_workflow(
        &par,
        &file,
        &provider,
        &executor,
        &[],
        &RunConfig::default(),
    )
    .await
    .expect("ok");
    assert!(r2.final_output.contains("parallel"));
}

// ── Conditional routing tests ────────────────────────────────────────────

fn make_conditional_workflow() -> (ReinFile, WorkflowDef) {
    let file = parse_file(
        r"
        agent triage { model: openai }
        agent respond { model: openai }
        agent escalate { model: openai }
    ",
    );

    let workflow = WorkflowDef {
        name: "support".to_string(),
        trigger: "ticket".to_string(),
        stages: vec![
            Stage {
                name: "triage".to_string(),
                agent: "triage".to_string(),
                route: RouteRule::Conditional {
                    field: "priority".to_string(),
                    equals: "high".to_string(),
                    then_stage: "escalate".to_string(),
                    else_stage: Some("respond".to_string()),
                },
                span: Span::new(0, 1),
            },
            Stage {
                name: "escalate".to_string(),
                agent: "escalate".to_string(),
                route: RouteRule::Next,
                span: Span::new(0, 1),
            },
            Stage {
                name: "respond".to_string(),
                agent: "respond".to_string(),
                route: RouteRule::Next,
                span: Span::new(0, 1),
            },
        ],
        mode: ExecutionMode::Sequential,
        span: Span::new(0, 1),
    };

    (file, workflow)
}

#[tokio::test]
async fn conditional_routes_to_then_stage() {
    let (file, workflow) = make_conditional_workflow();
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // triage → escalate (conditional match) → respond (Next from escalate)
    provider.push_response(simple_response("Priority: high. Urgent billing issue."));
    provider.push_response(simple_response("Escalated to manager."));
    provider.push_response(simple_response("Final response after escalation."));

    let result = run_sequential(
        &workflow,
        &file,
        &provider,
        &executor,
        &[],
        &RunConfig::default(),
    )
    .await
    .expect("ok");

    assert_eq!(result.stage_results.len(), 3);
    assert_eq!(result.stage_results[0].stage_name, "triage");
    assert_eq!(result.stage_results[1].stage_name, "escalate");
    assert_eq!(result.stage_results[2].stage_name, "respond");
    assert_eq!(result.final_output, "Final response after escalation.");
}

#[tokio::test]
async fn conditional_routes_to_else_stage() {
    let (file, workflow) = make_conditional_workflow();
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    provider.push_response(simple_response("Priority: low. Simple question."));
    provider.push_response(simple_response("Here's your answer."));

    let result = run_sequential(
        &workflow,
        &file,
        &provider,
        &executor,
        &[],
        &RunConfig::default(),
    )
    .await
    .expect("ok");

    assert_eq!(result.stage_results.len(), 2);
    assert_eq!(result.stage_results[0].stage_name, "triage");
    assert_eq!(result.stage_results[1].stage_name, "respond");
    assert_eq!(result.final_output, "Here's your answer.");
}

#[tokio::test]
async fn conditional_no_else_ends_workflow() {
    let file = parse_file(
        r"
        agent checker { model: openai }
        agent handler { model: openai }
    ",
    );

    let workflow = WorkflowDef {
        name: "check".to_string(),
        trigger: "event".to_string(),
        stages: vec![
            Stage {
                name: "checker".to_string(),
                agent: "checker".to_string(),
                route: RouteRule::Conditional {
                    field: "needs_action".to_string(),
                    equals: "yes".to_string(),
                    then_stage: "handler".to_string(),
                    else_stage: None,
                },
                span: Span::new(0, 1),
            },
            Stage {
                name: "handler".to_string(),
                agent: "handler".to_string(),
                route: RouteRule::Next,
                span: Span::new(0, 1),
            },
        ],
        mode: ExecutionMode::Sequential,
        span: Span::new(0, 1),
    };

    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    provider.push_response(simple_response("needs_action: no. All clear."));

    let result = run_sequential(
        &workflow,
        &file,
        &provider,
        &executor,
        &[],
        &RunConfig::default(),
    )
    .await
    .expect("ok");

    assert_eq!(result.stage_results.len(), 1);
    assert_eq!(result.stage_results[0].stage_name, "checker");
}
