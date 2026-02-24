use super::*;
use crate::ast::{
    AutoResolveBlock, AutoResolveCondition, CompareOp, ConditionMatcher, ExecutionMode, RouteRule,
    Span, Stage, StepDef, WhenComparison, WhenValue, WorkflowDef,
};
use crate::runtime::executor::MockExecutor;
use crate::runtime::provider::{ChatResponse, MockProvider, Usage};
use tempfile::NamedTempFile;

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
async fn single_stage_workflow() {
    let file = parse_file(
        r"
        agent triage { model: openai can [ zendesk.read_ticket ] }
    ",
    );
    let workflow = make_workflow("pipe", "ticket", &["triage"]);
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

    provider.push_response(simple_response("Triaged: low priority"));

    let result = run_sequential(&workflow, &ctx)
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

    // Stage 1: triage
    provider.push_response(simple_response("Priority: high. Issue: billing error."));
    // Stage 2: responder (receives triage output as input)
    provider.push_response(simple_response(
        "Dear customer, we've fixed your billing issue.",
    ));

    let result = run_sequential(&workflow, &ctx)
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

    provider.push_response(simple_response("output_a"));
    provider.push_response(simple_response("output_b"));
    provider.push_response(simple_response("output_c"));

    let result = run_sequential(&workflow, &ctx)
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

    provider.push_response(simple_response("ok"));

    let err = run_sequential(&workflow, &ctx).await.unwrap_err();

    assert!(err.to_string().contains("nonexistent"), "err: {err}");
}

#[tokio::test]
async fn stage_failure_returns_error() {
    let file = parse_file("agent a { model: openai }");
    let workflow = make_workflow("pipe", "event", &["a"]);
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

    provider.push_error("provider down");

    let err = run_sequential(&workflow, &ctx).await.unwrap_err();

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

    provider.push_response(simple_response("output_a"));
    provider.push_response(simple_response("output_b"));

    let result = run_parallel(&workflow, &ctx).await.expect("should succeed");

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

    // Queue response for agent "a" so it doesn't fail first
    provider.push_response(simple_response("ok"));

    let err = run_parallel(&workflow, &ctx).await.unwrap_err();

    assert!(err.to_string().contains("missing"), "err: {err}");
}

// #351: run_parallel must produce results in stage-declaration order even when
// stages complete concurrently. This verifies the ordering guarantee of the
// concurrent implementation.
#[tokio::test]
async fn parallel_workflow_preserves_stage_order() {
    let file = parse_file(
        r"
        agent a { model: openai }
        agent b { model: openai }
        agent c { model: openai }
    ",
    );
    let mut workflow = make_workflow("pipe", "event", &["a", "b", "c"]);
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

    provider.push_response(simple_response("first"));
    provider.push_response(simple_response("second"));
    provider.push_response(simple_response("third"));

    let result = run_parallel(&workflow, &ctx).await.expect("should succeed");

    assert_eq!(result.stage_results.len(), 3);
    // Results must be in stage-declaration order, not completion order.
    assert_eq!(result.stage_results[0].stage_name, "a");
    assert_eq!(result.stage_results[1].stage_name, "b");
    assert_eq!(result.stage_results[2].stage_name, "c");
    assert!(result.final_output.contains("first"));
    assert!(result.final_output.contains("second"));
    assert!(result.final_output.contains("third"));
}

#[tokio::test]
async fn run_workflow_dispatches_by_mode() {
    let file = parse_file("agent a { model: openai }");
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

    // Sequential
    let mut seq = make_workflow("seq", "event", &["a"]);
    seq.mode = ExecutionMode::Sequential;
    provider.push_response(simple_response("sequential"));
    let r1 = run_workflow(&seq, &ctx).await.expect("ok");
    assert_eq!(r1.final_output, "sequential");

    // Parallel
    let mut par = make_workflow("par", "event", &["a"]);
    par.mode = ExecutionMode::Parallel;
    provider.push_response(simple_response("parallel"));
    let r2 = run_workflow(&par, &ctx).await.expect("ok");
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
                    matcher: ConditionMatcher::Equals("high".to_string()),
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
        steps: vec![],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: None,
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: Span::new(0, 1),
    };

    (file, workflow)
}

#[tokio::test]
async fn conditional_routes_to_then_stage() {
    let (file, workflow) = make_conditional_workflow();
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

    // triage → escalate (conditional match) → respond (Next from escalate)
    provider.push_response(simple_response("Priority: high. Urgent billing issue."));
    provider.push_response(simple_response("Escalated to manager."));
    provider.push_response(simple_response("Final response after escalation."));

    let result = run_sequential(&workflow, &ctx).await.expect("ok");

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

    provider.push_response(simple_response("Priority: low. Simple question."));
    provider.push_response(simple_response("Here's your answer."));

    let result = run_sequential(&workflow, &ctx).await.expect("ok");

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
                    matcher: ConditionMatcher::Equals("yes".to_string()),
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
        steps: vec![],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: None,
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: Span::new(0, 1),
    };

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

    provider.push_response(simple_response("needs_action: no. All clear."));

    let result = run_sequential(&workflow, &ctx).await.expect("ok");

    assert_eq!(result.stage_results.len(), 1);
    assert_eq!(result.stage_results[0].stage_name, "checker");
}

// ── Resumable workflow tests ─────────────────────────────────────────────

#[tokio::test]
async fn resumable_fresh_run_no_checkpoint() {
    let file = parse_file(
        r"
        agent a { model: openai }
        agent b { model: openai }
    ",
    );
    let workflow = make_workflow("pipe", "event", &["a", "b"]);
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

    provider.push_response(simple_response("output_a"));
    provider.push_response(simple_response("output_b"));

    let tmp = NamedTempFile::new().unwrap();
    let state_path = tmp.path().to_path_buf();
    drop(tmp); // no checkpoint — path now points to a nonexistent file

    let result = run_sequential_resumable(&workflow, &ctx, &state_path)
        .await
        .expect("should succeed");

    assert_eq!(result.stage_results.len(), 2);
    assert_eq!(result.final_output, "output_b");
    assert!(
        !state_path.exists(),
        "state file should be cleaned up on success"
    );
}

#[tokio::test]
async fn resumable_resumes_after_first_stage() {
    let file = parse_file(
        r"
        agent a { model: openai }
        agent b { model: openai }
    ",
    );
    let workflow = make_workflow("pipe", "event", &["a", "b"]);
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

    // Only stage b gets a response — if stage a runs it would consume this
    // response and stage b would fail with an empty queue.
    provider.push_response(simple_response("output_b"));

    let tmp = NamedTempFile::new().unwrap();
    let state_path = tmp.path().to_path_buf();
    persistence::save_state(
        &persistence::WorkflowState {
            version: persistence::WORKFLOW_STATE_VERSION,
            workflow_name: "pipe".to_string(),
            completed_stages: vec![persistence::CompletedStage {
                stage_name: "a".to_string(),
                agent_name: "a".to_string(),
                output: "output_a".to_string(),
                cost_cents: 5,
                tokens: 100,
            }],
            next_input: "output_a".to_string(),
        },
        &state_path,
    )
    .unwrap();

    let result = run_sequential_resumable(&workflow, &ctx, &state_path)
        .await
        .expect("should succeed");

    assert_eq!(result.stage_results.len(), 2);
    assert_eq!(result.stage_results[0].stage_name, "a");
    assert_eq!(result.stage_results[0].output, "output_a");
    assert_eq!(result.stage_results[1].stage_name, "b");
    assert_eq!(result.stage_results[1].output, "output_b");
    assert_eq!(result.final_output, "output_b");
    assert!(
        !state_path.exists(),
        "state file should be cleaned up on success"
    );
}

#[tokio::test]
async fn resumable_resumes_mid_pipeline() {
    let file = parse_file(
        r"
        agent a { model: openai }
        agent b { model: openai }
        agent c { model: openai }
        agent d { model: openai }
    ",
    );
    let workflow = make_workflow("pipe", "event", &["a", "b", "c", "d"]);
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

    // Only c and d get responses — a and b are replayed from the checkpoint.
    provider.push_response(simple_response("output_c"));
    provider.push_response(simple_response("output_d"));

    let tmp = NamedTempFile::new().unwrap();
    let state_path = tmp.path().to_path_buf();
    persistence::save_state(
        &persistence::WorkflowState {
            version: persistence::WORKFLOW_STATE_VERSION,
            workflow_name: "pipe".to_string(),
            completed_stages: vec![
                persistence::CompletedStage {
                    stage_name: "a".to_string(),
                    agent_name: "a".to_string(),
                    output: "output_a".to_string(),
                    cost_cents: 3,
                    tokens: 50,
                },
                persistence::CompletedStage {
                    stage_name: "b".to_string(),
                    agent_name: "b".to_string(),
                    output: "output_b".to_string(),
                    cost_cents: 4,
                    tokens: 60,
                },
            ],
            next_input: "output_b".to_string(),
        },
        &state_path,
    )
    .unwrap();

    let result = run_sequential_resumable(&workflow, &ctx, &state_path)
        .await
        .expect("should succeed");

    assert_eq!(result.stage_results.len(), 4);
    assert_eq!(result.stage_results[0].stage_name, "a");
    assert_eq!(result.stage_results[1].stage_name, "b");
    assert_eq!(result.stage_results[2].stage_name, "c");
    assert_eq!(result.stage_results[3].stage_name, "d");
    assert_eq!(result.final_output, "output_d");
    assert!(!state_path.exists());
}

#[tokio::test]
async fn resumable_different_workflow_name_restarts_fresh() {
    let file = parse_file(
        r"
        agent a { model: openai }
        agent b { model: openai }
    ",
    );
    let workflow = make_workflow("workflow_b", "event", &["a", "b"]);
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

    // Both stages must run — the checkpoint is for a different workflow.
    provider.push_response(simple_response("output_a"));
    provider.push_response(simple_response("output_b"));

    let tmp = NamedTempFile::new().unwrap();
    let state_path = tmp.path().to_path_buf();
    persistence::save_state(
        &persistence::WorkflowState {
            version: persistence::WORKFLOW_STATE_VERSION,
            workflow_name: "workflow_a".to_string(),
            completed_stages: vec![persistence::CompletedStage {
                stage_name: "a".to_string(),
                agent_name: "a".to_string(),
                output: "stale_output".to_string(),
                cost_cents: 1,
                tokens: 10,
            }],
            next_input: "stale_output".to_string(),
        },
        &state_path,
    )
    .unwrap();

    let result = run_sequential_resumable(&workflow, &ctx, &state_path)
        .await
        .expect("should succeed");

    assert_eq!(result.stage_results.len(), 2);
    assert_eq!(result.stage_results[0].output, "output_a");
    assert_eq!(result.final_output, "output_b");
}

#[tokio::test]
async fn resumable_conditional_routing_on_resume() {
    let (file, workflow) = make_conditional_workflow();
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

    // Triage is in the checkpoint; only escalate and respond need responses.
    provider.push_response(simple_response("Escalated to manager."));
    provider.push_response(simple_response("Final response after escalation."));

    let tmp = NamedTempFile::new().unwrap();
    let state_path = tmp.path().to_path_buf();
    // Checkpoint: triage completed with a "priority: high" output.
    // find_resume_start must replay this condition and route to escalate.
    persistence::save_state(
        &persistence::WorkflowState {
            version: persistence::WORKFLOW_STATE_VERSION,
            workflow_name: "support".to_string(),
            completed_stages: vec![persistence::CompletedStage {
                stage_name: "triage".to_string(),
                agent_name: "triage".to_string(),
                output: "Priority: high. Urgent billing issue.".to_string(),
                cost_cents: 5,
                tokens: 100,
            }],
            next_input: "Priority: high. Urgent billing issue.".to_string(),
        },
        &state_path,
    )
    .unwrap();

    let result = run_sequential_resumable(&workflow, &ctx, &state_path)
        .await
        .expect("should succeed");

    // triage (checkpoint) → escalate (condition matched) → respond (Next)
    assert_eq!(result.stage_results.len(), 3);
    assert_eq!(result.stage_results[0].stage_name, "triage");
    assert_eq!(result.stage_results[1].stage_name, "escalate");
    assert_eq!(result.stage_results[2].stage_name, "respond");
    assert_eq!(result.final_output, "Final response after escalation.");
}

#[tokio::test]
async fn resumable_corrupt_checkpoint_returns_persistence_error() {
    let file = parse_file("agent a { model: openai }");
    let workflow = make_workflow("pipe", "event", &["a"]);
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

    let tmp = NamedTempFile::new().unwrap();
    let state_path = tmp.path().to_path_buf();
    std::fs::write(&state_path, "not valid json {{{").unwrap();

    let err = run_sequential_resumable(&workflow, &ctx, &state_path)
        .await
        .unwrap_err();

    assert!(
        matches!(err, WorkflowError::PersistenceFailure(_)),
        "expected PersistenceFailure, got: {err}"
    );
}

// ── Review feedback tests ────────────────────────────────────────────────

#[tokio::test]
async fn condition_match_rejects_prefix_false_positive() {
    let eq_high = ConditionMatcher::Equals("high".to_string());
    // "high" should NOT match "higher"
    assert!(condition_matches("priority: high", "priority", &eq_high));
    assert!(condition_matches("priority: high.", "priority", &eq_high));
    assert!(!condition_matches("priority: higher", "priority", &eq_high));
    assert!(!condition_matches(
        "priority: highlights",
        "priority",
        &eq_high,
    ));

    // Contains matcher
    let contains = ConditionMatcher::Contains("bill".to_string());
    assert!(condition_matches(
        "category: billing issue",
        "category",
        &contains
    ));
    assert!(!condition_matches(
        "category: technical",
        "category",
        &contains
    ));

    // Regex matcher
    let regex = ConditionMatcher::Regex(r"^(high|critical)$".to_string());
    assert!(condition_matches("priority: high", "priority", &regex));
    assert!(condition_matches("priority: critical", "priority", &regex));
    assert!(!condition_matches("priority: low", "priority", &regex));

    // Invalid regex doesn't panic, just returns false
    let bad_regex = ConditionMatcher::Regex(r"[invalid".to_string());
    assert!(!condition_matches("priority: high", "priority", &bad_regex));
}

#[tokio::test]
async fn condition_match_json_path() {
    let json_output = r#"{"result": {"status": "escalated", "score": 95}}"#;

    let jp = ConditionMatcher::JsonPath {
        path: "result.status".to_string(),
        expected: "escalated".to_string(),
    };
    assert!(condition_matches(json_output, "", &jp));

    let jp_num = ConditionMatcher::JsonPath {
        path: "result.score".to_string(),
        expected: "95".to_string(),
    };
    assert!(condition_matches(json_output, "", &jp_num));

    let jp_miss = ConditionMatcher::JsonPath {
        path: "result.status".to_string(),
        expected: "resolved".to_string(),
    };
    assert!(!condition_matches(json_output, "", &jp_miss));

    // Non-JSON output returns false
    let jp2 = ConditionMatcher::JsonPath {
        path: "status".to_string(),
        expected: "ok".to_string(),
    };
    assert!(!condition_matches("not json at all", "", &jp2));
}

#[tokio::test]
async fn conditional_route_to_nonexistent_stage_errors() {
    let file = parse_file(
        r"
        agent triage { model: openai }
    ",
    );

    let workflow = WorkflowDef {
        name: "bad".to_string(),
        trigger: "event".to_string(),
        stages: vec![Stage {
            name: "triage".to_string(),
            agent: "triage".to_string(),
            route: RouteRule::Conditional {
                field: "priority".to_string(),
                matcher: ConditionMatcher::Equals("high".to_string()),
                then_stage: "nonexistent".to_string(),
                else_stage: None,
            },
            span: Span::new(0, 1),
        }],
        steps: vec![],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: None,
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: Span::new(0, 1),
    };

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
    provider.push_response(simple_response("priority: high"));

    let err = run_sequential(&workflow, &ctx).await.unwrap_err();

    assert!(
        matches!(err, WorkflowError::StageNotFound(ref name) if name == "nonexistent"),
        "expected StageNotFound, got: {err}"
    );
}

#[tokio::test]
async fn circular_route_returns_error() {
    let file = parse_file(
        r"
        agent a { model: openai }
        agent b { model: openai }
    ",
    );

    // a routes to b, b routes to a → circular
    let workflow = WorkflowDef {
        name: "loop".to_string(),
        trigger: "event".to_string(),
        stages: vec![
            Stage {
                name: "a".to_string(),
                agent: "a".to_string(),
                route: RouteRule::Conditional {
                    field: "go".to_string(),
                    matcher: ConditionMatcher::Equals("yes".to_string()),
                    then_stage: "b".to_string(),
                    else_stage: None,
                },
                span: Span::new(0, 1),
            },
            Stage {
                name: "b".to_string(),
                agent: "b".to_string(),
                route: RouteRule::Conditional {
                    field: "go".to_string(),
                    matcher: ConditionMatcher::Equals("yes".to_string()),
                    then_stage: "a".to_string(),
                    else_stage: None,
                },
                span: Span::new(0, 1),
            },
        ],
        steps: vec![],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: None,
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: Span::new(0, 1),
    };

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
    provider.push_response(simple_response("go: yes"));
    provider.push_response(simple_response("go: yes"));

    let err = run_sequential(&workflow, &ctx).await.unwrap_err();

    assert!(
        matches!(err, WorkflowError::CircularRoute(ref name) if name == "a"),
        "expected CircularRoute, got: {err}"
    );
}

#[tokio::test]
async fn step_execution_runs_agent_with_goal() {
    use crate::ast::StepDef;
    let file = parse_file(
        r#"
        agent writer { model: openai can [ docs.write ] }
    "#,
    );

    let workflow = WorkflowDef {
        name: "test_wf".to_string(),
        trigger: "new_doc".to_string(),
        stages: vec![],
        steps: vec![StepDef {
            name: "draft".to_string(),
            agent: "writer".to_string(),
            goal: Some("Write a first draft".to_string()),
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
            approval: None,
            span: Span::new(0, 1),
        }],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: None,
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: Span::new(0, 1),
    };

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

    provider.push_response(simple_response("Draft complete!"));

    let result = run_workflow(&workflow, &ctx).await.unwrap();
    assert_eq!(result.stage_results.len(), 1);
    assert_eq!(result.stage_results[0].stage_name, "draft");
    assert_eq!(result.stage_results[0].agent_name, "writer");
    assert_eq!(result.final_output, "Draft complete!");
}

// --- #301 Approval Handler Tests ---

#[tokio::test]
async fn step_with_auto_approve_proceeds() {
    use crate::ast::{ApprovalDef, ApprovalKind, Span as AstSpan};
    use crate::runtime::approval::AutoApproveHandler;
    use std::sync::Arc;

    let file = parse_file(
        r#"
        agent writer { model: openai }
    "#,
    );
    let workflow = WorkflowDef {
        name: "approval_test".to_string(),
        trigger: "start".to_string(),
        stages: vec![],
        steps: vec![crate::ast::StepDef {
            name: "draft".to_string(),
            agent: "writer".to_string(),
            goal: Some("Write a draft".to_string()),
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
                channel: "cli".to_string(),
                destination: "#ops".to_string(),
                timeout: Some("1h".to_string()),
                mode: None,
                span: AstSpan::new(0, 1),
            }),
            span: AstSpan::new(0, 1),
        }],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: None,
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: Span::new(0, 1),
    };

    let provider = MockProvider::new();
    let executor = MockExecutor::new();
    let ctx = WorkflowContext {
        file: &file,
        provider: &provider,
        executor: &executor,
        tool_defs: &[],
        config: &RunConfig::default(),
        approval_handler: Some(Arc::new(AutoApproveHandler)),
        audit_log: None,
        workflow_name: None,
    };

    provider.push_response(simple_response("Draft approved and complete"));

    let result = run_workflow(&workflow, &ctx).await.unwrap();
    assert_eq!(result.final_output, "Draft approved and complete");
}

#[tokio::test]
async fn step_with_auto_reject_returns_error() {
    use crate::ast::{ApprovalDef, ApprovalKind, Span as AstSpan};
    use crate::runtime::approval::AutoRejectHandler;
    use std::sync::Arc;

    let file = parse_file(
        r#"
        agent writer { model: openai }
    "#,
    );
    let workflow = WorkflowDef {
        name: "reject_test".to_string(),
        trigger: "start".to_string(),
        stages: vec![],
        steps: vec![crate::ast::StepDef {
            name: "risky_step".to_string(),
            agent: "writer".to_string(),
            goal: Some("Do something risky".to_string()),
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
                channel: "cli".to_string(),
                destination: "#ops".to_string(),
                timeout: Some("1h".to_string()),
                mode: None,
                span: AstSpan::new(0, 1),
            }),
            span: AstSpan::new(0, 1),
        }],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: None,
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: Span::new(0, 1),
    };

    let provider = MockProvider::new();
    let executor = MockExecutor::new();
    let ctx = WorkflowContext {
        file: &file,
        provider: &provider,
        executor: &executor,
        tool_defs: &[],
        config: &RunConfig::default(),
        approval_handler: Some(Arc::new(AutoRejectHandler::new("test rejection"))),
        audit_log: None,
        workflow_name: None,
    };

    let result = run_workflow(&workflow, &ctx).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("rejected"),
        "error should mention rejection: {err}"
    );
}

#[tokio::test]
async fn step_without_approval_def_skips_handler() {
    let file = parse_file(
        r#"
        agent writer { model: openai }
    "#,
    );
    let workflow = WorkflowDef {
        name: "no_approval_test".to_string(),
        trigger: "start".to_string(),
        stages: vec![],
        steps: vec![crate::ast::StepDef {
            name: "simple_step".to_string(),
            agent: "writer".to_string(),
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
            approval: None,
            span: Span::new(0, 1),
        }],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: None,
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: Span::new(0, 1),
    };

    let provider = MockProvider::new();
    let executor = MockExecutor::new();
    // No approval handler — should not crash
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

    provider.push_response(simple_response("Done without approval"));

    let result = run_workflow(&workflow, &ctx).await.unwrap();
    assert_eq!(result.final_output, "Done without approval");
}

// --- #358 Audit log wiring tests ---

#[tokio::test]
async fn step_with_audit_log_records_approval_events() {
    use crate::ast::{ApprovalDef, ApprovalKind, Span as AstSpan};
    use crate::runtime::approval::AutoApproveHandler;
    use crate::runtime::audit::{AuditKind, AuditLog};
    use std::sync::Arc;

    let tmp = tempfile::NamedTempFile::new().expect("temp file");
    let log = Arc::new(AuditLog::new(tmp.path()).expect("AuditLog::new"));

    let file = parse_file(r#"agent writer { model: openai }"#);
    let workflow = WorkflowDef {
        name: "audit_test_workflow".to_string(),
        trigger: "start".to_string(),
        stages: vec![],
        steps: vec![crate::ast::StepDef {
            name: "gated_step".to_string(),
            agent: "writer".to_string(),
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
                channel: "cli".to_string(),
                destination: "#ops".to_string(),
                timeout: None,
                mode: None,
                span: AstSpan::new(0, 1),
            }),
            span: AstSpan::new(0, 1),
        }],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: None,
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: Span::new(0, 1),
    };

    let provider = MockProvider::new();
    provider.push_response(simple_response("approved output"));
    let executor = MockExecutor::new();
    let ctx = WorkflowContext {
        file: &file,
        provider: &provider,
        executor: &executor,
        tool_defs: &[],
        config: &RunConfig::default(),
        approval_handler: Some(Arc::new(AutoApproveHandler)),
        audit_log: Some(Arc::clone(&log)),
        workflow_name: Some("audit_test_workflow".to_string()),
    };

    let result = run_workflow(&workflow, &ctx)
        .await
        .expect("workflow should succeed");
    assert_eq!(result.final_output, "approved output");

    // The audit log must contain exactly two entries: ApprovalRequested + ApprovalResolved.
    let entries = log.read_all().expect("read audit log");
    assert_eq!(
        entries.len(),
        2,
        "expected 2 audit entries, got: {entries:#?}"
    );
    assert!(
        entries
            .iter()
            .any(|e| e.kind == AuditKind::ApprovalRequested),
        "missing ApprovalRequested entry"
    );
    assert!(
        entries
            .iter()
            .any(|e| e.kind == AuditKind::ApprovalResolved),
        "missing ApprovalResolved entry"
    );
    // Both entries must reference the correct workflow and step.
    for entry in &entries {
        assert_eq!(entry.workflow.as_deref(), Some("audit_test_workflow"));
        assert_eq!(entry.step.as_deref(), Some("gated_step"));
    }
}

/// #358 — When `audit_log` is `Some` but `workflow_name` is `None`, the audit
/// entries must have `workflow == None`, not `workflow == Some("")`. An empty
/// string in the `workflow` field would cause compliance consumers to treat
/// "no workflow" as a real (but unnamed) workflow.
#[tokio::test]
async fn step_with_audit_log_and_no_workflow_name_omits_workflow_field() {
    use crate::ast::{ApprovalDef, ApprovalKind, Span as AstSpan};
    use crate::runtime::approval::AutoApproveHandler;
    use crate::runtime::audit::AuditLog;
    use std::sync::Arc;

    let tmp = tempfile::NamedTempFile::new().expect("temp file");
    let log = Arc::new(AuditLog::new(tmp.path()).expect("AuditLog::new"));

    let file = parse_file(r#"agent writer { model: openai }"#);
    let workflow = WorkflowDef {
        name: "no_name_workflow".to_string(),
        trigger: "start".to_string(),
        stages: vec![],
        steps: vec![crate::ast::StepDef {
            name: "gated_step".to_string(),
            agent: "writer".to_string(),
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
                channel: "cli".to_string(),
                destination: "#ops".to_string(),
                timeout: None,
                mode: None,
                span: AstSpan::new(0, 1),
            }),
            span: AstSpan::new(0, 1),
        }],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: None,
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: Span::new(0, 1),
    };

    let provider = MockProvider::new();
    provider.push_response(simple_response("output"));
    let executor = MockExecutor::new();
    let ctx = WorkflowContext {
        file: &file,
        provider: &provider,
        executor: &executor,
        tool_defs: &[],
        config: &RunConfig::default(),
        approval_handler: Some(Arc::new(AutoApproveHandler)),
        audit_log: Some(Arc::clone(&log)),
        workflow_name: None, // intentionally absent
    };

    run_workflow(&workflow, &ctx)
        .await
        .expect("workflow should succeed");

    let entries = log.read_all().expect("read audit log");
    assert_eq!(
        entries.len(),
        2,
        "expected ApprovalRequested + ApprovalResolved"
    );
    for entry in &entries {
        assert!(
            entry.workflow.is_none(),
            "workflow field must be None when workflow_name is not set, got: {:?}",
            entry.workflow
        );
    }
}

/// #358 — When `audit_log` is `None`, the inner handler is called exactly once.
/// This pins the no-audit code path: `run_step` must not wrap the handler or
/// call it more than once when no audit log is configured.
#[tokio::test]
async fn step_without_audit_log_calls_handler_exactly_once() {
    use crate::ast::{ApprovalDef, ApprovalKind, Span as AstSpan};
    use crate::runtime::approval::{ApprovalHandler, ApprovalStatus};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingHandler(Arc<AtomicUsize>);

    #[async_trait::async_trait]
    impl ApprovalHandler for CountingHandler {
        async fn request_approval(
            &self,
            _step: &str,
            _output: &str,
            _approval: &crate::ast::ApprovalDef,
        ) -> ApprovalStatus {
            self.0.fetch_add(1, Ordering::Relaxed);
            ApprovalStatus::Approved
        }
    }

    let call_count = Arc::new(AtomicUsize::new(0));
    let handler: Arc<dyn ApprovalHandler> = Arc::new(CountingHandler(Arc::clone(&call_count)));

    let file = parse_file(r#"agent writer { model: openai }"#);
    let workflow = WorkflowDef {
        name: "no_audit_workflow".to_string(),
        trigger: "start".to_string(),
        stages: vec![],
        steps: vec![crate::ast::StepDef {
            name: "gated_step".to_string(),
            agent: "writer".to_string(),
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
                channel: "cli".to_string(),
                destination: String::new(),
                timeout: None,
                mode: None,
                span: AstSpan::new(0, 1),
            }),
            span: AstSpan::new(0, 1),
        }],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: None,
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: Span::new(0, 1),
    };

    let provider = MockProvider::new();
    provider.push_response(simple_response("done"));
    let executor = MockExecutor::new();
    let ctx = WorkflowContext {
        file: &file,
        provider: &provider,
        executor: &executor,
        tool_defs: &[],
        config: &RunConfig::default(),
        approval_handler: Some(handler),
        audit_log: None, // no audit log — no AuditingApprovalHandler wrapping
        workflow_name: None,
    };

    run_workflow(&workflow, &ctx)
        .await
        .expect("workflow should succeed");

    // The handler must be called exactly once — not zero times (skipped) or
    // twice (double-delegation bug).
    assert_eq!(
        call_count.load(Ordering::Relaxed),
        1,
        "inner handler must be called exactly once when audit_log is None"
    );
}

/// #474 — When `approval_handler` is `None` and `audit_log` is `Some`, `run_step`
/// must call `resolve_approval_handler` (not panic or skip auditing), wrap the
/// resolved handler in `AuditingApprovalHandler`, and write audit entries.
///
/// This is the production path for users who pass `--audit-log` without injecting
/// a handler — they rely entirely on `resolve_approval_handler` + the wrapping logic.
/// Previous tests always set `approval_handler: Some(AutoApproveHandler)`, leaving
/// the `Arc::from(Box<dyn ApprovalHandler>)` conversion branch uncovered by CI.
#[tokio::test]
async fn step_with_audit_log_and_no_injected_handler_uses_resolved_handler() {
    use crate::ast::{ApprovalDef, ApprovalKind, Span as AstSpan};
    use crate::runtime::audit::{AuditKind, AuditLog};
    use std::sync::Arc;

    let tmp = tempfile::NamedTempFile::new().expect("temp file");
    let log = Arc::new(AuditLog::new(tmp.path()).expect("AuditLog::new"));

    let file = parse_file(r#"agent writer { model: openai }"#);
    let workflow = WorkflowDef {
        name: "no_handler_wf".to_string(),
        trigger: "start".to_string(),
        stages: vec![],
        steps: vec![crate::ast::StepDef {
            name: "gated".to_string(),
            agent: "writer".to_string(),
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
                // "cli" routes through resolve_approval_handler → CliApprovalHandler.
                // CliApprovalHandler reads stdin; in non-interactive CI stdin is EOF
                // so it returns ApprovalRejected. That is expected — the test goal is
                // to confirm the Arc::from + AuditingApprovalHandler wrapping path.
                channel: "cli".to_string(),
                destination: "#ops".to_string(),
                timeout: None,
                mode: None,
                span: AstSpan::new(0, 1),
            }),
            span: AstSpan::new(0, 1),
        }],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: None,
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: Span::new(0, 1),
    };

    let provider = MockProvider::new();
    provider.push_response(simple_response("resolved output"));
    let executor = MockExecutor::new();
    let ctx = WorkflowContext {
        file: &file,
        provider: &provider,
        executor: &executor,
        tool_defs: &[],
        config: &RunConfig::default(),
        // No injected handler — must fall through to resolve_approval_handler.
        approval_handler: None,
        audit_log: Some(Arc::clone(&log)),
        workflow_name: Some("no_handler_wf".to_string()),
    };

    // The resolved CliApprovalHandler reads stdin; in non-interactive CI, stdin
    // is EOF so read_line returns Ok(0) → empty string → Rejected. Wrap in a
    // timeout so the test fails fast (with a clear message) rather than hanging
    // if stdin is not closed in some CI configurations.
    //
    // The goal is not to verify approval outcome but to confirm:
    //   (a) no panic from the Arc::from(Box<dyn ApprovalHandler>) conversion,
    //   (b) AuditingApprovalHandler wraps the resolved handler and writes entries.
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        run_workflow(&workflow, &ctx),
    )
    .await
    .expect("test timed out — CliApprovalHandler may be blocking on stdin");

    // CliApprovalHandler rejects on EOF stdin; assert the specific error so a
    // failure here (e.g., AgentNotFound before approval) surfaces clearly.
    assert!(
        matches!(result, Err(WorkflowError::ApprovalRejected { .. })),
        "expected ApprovalRejected from CliApprovalHandler on empty stdin; got: {result:?}"
    );

    // Both audit entries must be present even on rejection: AuditingApprovalHandler
    // writes ApprovalRequested before delegating and ApprovalResolved after.
    let entries = log.read_all().expect("read audit log");
    assert_eq!(
        entries.len(),
        2,
        "expected ApprovalRequested + ApprovalResolved; got: {entries:#?}"
    );
    assert!(
        entries
            .iter()
            .any(|e| e.kind == AuditKind::ApprovalRequested),
        "missing ApprovalRequested entry"
    );
    assert!(
        entries
            .iter()
            .any(|e| e.kind == AuditKind::ApprovalResolved),
        "missing ApprovalResolved entry"
    );
    for entry in &entries {
        assert_eq!(entry.workflow.as_deref(), Some("no_handler_wf"));
        assert_eq!(entry.step.as_deref(), Some("gated"));
    }
}

// --- #303 DAG depends_on Tests ---

fn make_step(name: &str, agent: &str, depends_on: Vec<&str>) -> StepDef {
    StepDef {
        name: name.to_string(),
        agent: agent.to_string(),
        goal: None,
        input: None,
        output_constraints: vec![],
        depends_on: depends_on.into_iter().map(str::to_string).collect(),
        when: None,
        on_failure: None,
        send_to: None,
        fallback: None,
        for_each: None,
        typed_input: None,
        typed_outputs: vec![],
        escalate: None,
        approval: None,
        span: Span::new(0, 1),
    }
}

#[test]
fn dag_no_deps_preserves_file_order() {
    let steps = vec![make_step("a", "bot", vec![]), make_step("b", "bot", vec![])];
    let order = resolve_dag(&steps).expect("no cycle");
    let names: Vec<&str> = order.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["a", "b"]);
}

#[test]
fn dag_single_dependency_reorders() {
    // b depends on a — even if b comes first in file order, a must execute first
    let steps = vec![
        make_step("b", "bot", vec!["a"]),
        make_step("a", "bot", vec![]),
    ];
    let order = resolve_dag(&steps).expect("no cycle");
    let names: Vec<&str> = order.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names[0], "a", "a must come before b");
    assert_eq!(names[1], "b");
}

#[test]
fn dag_diamond_dependency_valid() {
    // a → b, a → c, b → d, c → d
    let steps = vec![
        make_step("a", "bot", vec![]),
        make_step("b", "bot", vec!["a"]),
        make_step("c", "bot", vec!["a"]),
        make_step("d", "bot", vec!["b", "c"]),
    ];
    let order = resolve_dag(&steps).expect("no cycle");
    let names: Vec<&str> = order.iter().map(|s| s.name.as_str()).collect();
    // a must come first, d must come last
    assert_eq!(names[0], "a");
    assert_eq!(names[names.len() - 1], "d");
    // b and c must appear before d
    let d_idx = names.iter().position(|&n| n == "d").unwrap();
    let b_idx = names.iter().position(|&n| n == "b").unwrap();
    let c_idx = names.iter().position(|&n| n == "c").unwrap();
    assert!(b_idx < d_idx);
    assert!(c_idx < d_idx);
}

#[test]
fn dag_cycle_returns_error() {
    // a → b → a forms a cycle
    let steps = vec![
        make_step("a", "bot", vec!["b"]),
        make_step("b", "bot", vec!["a"]),
    ];
    let err = resolve_dag(&steps).unwrap_err();
    assert!(
        err.to_string().contains("cycle") || err.to_string().contains("Cycle"),
        "error should mention cycle: {err}"
    );
}

#[tokio::test]
async fn workflow_steps_respect_depends_on_order() {
    // Steps declared out of order: b depends on a
    let file = parse_file(
        r#"
        agent bot { model: openai }
    "#,
    );
    let workflow = WorkflowDef {
        name: "dag_wf".to_string(),
        trigger: "start".to_string(),
        stages: vec![],
        steps: vec![
            make_step("b", "bot", vec!["a"]),
            make_step("a", "bot", vec![]),
        ],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: None,
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: Span::new(0, 1),
    };

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

    // a runs first, b runs second
    provider.push_response(simple_response("output from a"));
    provider.push_response(simple_response("output from b"));

    let result = run_workflow(&workflow, &ctx).await.unwrap();
    assert_eq!(result.stage_results.len(), 2);
    assert_eq!(result.final_output, "output from b");
}

// --- #307 Step Extensions Tests ---

fn make_step_with_fallback(name: &str, agent: &str, fallback_agent: &str) -> StepDef {
    let fallback = StepDef {
        name: format!("{name}_fallback"),
        agent: fallback_agent.to_string(),
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
        approval: None,
        span: Span::new(0, 1),
    };
    StepDef {
        name: name.to_string(),
        agent: agent.to_string(),
        goal: None,
        input: None,
        output_constraints: vec![],
        depends_on: vec![],
        when: None,
        on_failure: None,
        send_to: None,
        fallback: Some(Box::new(fallback)),
        for_each: None,
        typed_input: None,
        typed_outputs: vec![],
        escalate: None,
        approval: None,
        span: Span::new(0, 1),
    }
}

fn make_step_for_each(name: &str, agent: &str, collection: &str) -> StepDef {
    StepDef {
        name: name.to_string(),
        agent: agent.to_string(),
        goal: None,
        input: None,
        output_constraints: vec![],
        depends_on: vec![],
        when: None,
        on_failure: None,
        send_to: None,
        fallback: None,
        for_each: Some(collection.to_string()),
        typed_input: None,
        typed_outputs: vec![],
        escalate: None,
        approval: None,
        span: Span::new(0, 1),
    }
}

#[tokio::test]
async fn step_fallback_runs_on_primary_failure() {
    // "ghost" agent does not exist → primary step fails → fallback should run.
    let file = parse_file(r"agent backup { model: openai }");
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // Fallback agent "backup" will produce this response.
    provider.push_response(simple_response("fallback result"));

    let step = make_step_with_fallback("classify", "ghost", "backup");
    let workflow = WorkflowDef {
        name: "wf".to_string(),
        trigger: "test".to_string(),
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

    let (results, events) = run_steps(&workflow, &ctx).await.expect("should succeed");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].output, "fallback result");
    // StepFallback event must be emitted when fallback executes.
    assert!(
        events
            .iter()
            .any(|e| matches!(e, crate::runtime::RunEvent::StepFallback { .. })),
        "expected StepFallback event"
    );
}

#[tokio::test]
async fn step_without_fallback_propagates_error() {
    let file = parse_file(r"agent backup { model: openai }");
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // Step references nonexistent agent with no fallback.
    let step = make_step("alone", "ghost_no_fallback", vec![]);
    let workflow = WorkflowDef {
        name: "wf".to_string(),
        trigger: "test".to_string(),
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

    // Since #363: soft errors (AgentNotFound/StageFailed) no longer abort the
    // whole run — run_steps returns Ok so dependent steps can be skipped.
    // The failed step's result has empty output.
    let (results, events) = run_steps(&workflow, &ctx)
        .await
        .expect("soft errors return Ok");
    assert_eq!(results.len(), 1);
    assert!(
        results[0].output.is_empty(),
        "failed step output must be empty"
    );
    assert_eq!(
        results[0].status,
        StageResultStatus::Failed,
        "failed StageResult must have status Failed so consumers can distinguish it"
    );
    // A StepFailed event must be emitted so the trace is observable.
    assert!(
        events.iter().any(
            |e| matches!(e, crate::runtime::RunEvent::StepFailed { step, .. } if step == "alone")
        ),
        "expected StepFailed event for step 'alone'"
    );
}

#[tokio::test]
async fn step_for_each_iterates_over_array() {
    let file = parse_file(r"agent bot { model: openai }");
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // Three items → three responses.
    provider.push_response(simple_response("processed a"));
    provider.push_response(simple_response("processed b"));
    provider.push_response(simple_response("processed c"));

    let step = make_step_for_each("process_items", "bot", "items");
    let workflow = WorkflowDef {
        name: "wf".to_string(),
        trigger: "test".to_string(),
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

    // Input with a JSON array keyed by "items".
    let workflow_with_trigger = WorkflowDef {
        trigger: r#"{"items": ["a", "b", "c"]}"#.to_string(),
        ..workflow
    };

    let (results, events) = run_steps(&workflow_with_trigger, &ctx)
        .await
        .expect("should succeed");
    // One aggregated StageResult per for_each step with outputs as a JSON array.
    assert_eq!(results.len(), 1, "one result per for_each step");
    let output = &results[0].output;
    // Outputs are serialized as a JSON array to avoid newline ambiguity.
    assert!(output.contains("processed a"), "missing iteration 0");
    assert!(output.contains("processed b"), "missing iteration 1");
    assert!(output.contains("processed c"), "missing iteration 2");
    // One ForEachIteration event per item.
    let iter_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, crate::runtime::RunEvent::ForEachIteration { .. }))
        .collect();
    assert_eq!(iter_events.len(), 3, "expected 3 ForEachIteration events");
}

#[tokio::test]
async fn workflow_auto_resolve_short_circuits_on_condition_met() {
    // workflow.auto_resolve: when { confidence > 0 }
    // First step outputs JSON with confidence=100; remaining steps should be skipped.
    let file = parse_file(
        r#"
        agent bot { model: openai }
        agent should_not_run { model: openai }
        "#,
    );
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // Only bot responds; should_not_run must not be called.
    provider.push_response(simple_response(r#"{"confidence": 100}"#));

    let auto_resolve = AutoResolveBlock {
        conditions: vec![AutoResolveCondition::Comparison(WhenComparison {
            field: "confidence".to_string(),
            op: CompareOp::Gt,
            value: WhenValue::Number("0".to_string()),
        })],
        span: Span::new(0, 1),
    };

    let workflow = WorkflowDef {
        name: "wf".to_string(),
        trigger: "test".to_string(),
        stages: vec![],
        steps: vec![
            make_step("first", "bot", vec![]),
            make_step("second", "should_not_run", vec![]),
        ],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: Some(auto_resolve),
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: Span::new(0, 1),
    };
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

    let (results, events) = run_steps(&workflow, &ctx).await.expect("should succeed");
    // Only the first step ran; second was short-circuited.
    assert_eq!(results.len(), 1, "should stop after auto_resolve");
    assert_eq!(results[0].stage_name, "first");
    // AutoResolved event must be emitted.
    assert!(
        events
            .iter()
            .any(|e| matches!(e, crate::runtime::RunEvent::AutoResolved { .. })),
        "expected AutoResolved event"
    );
}

#[tokio::test]
async fn workflow_auto_resolve_does_not_short_circuit_when_condition_unmet() {
    let file = parse_file(
        r#"
        agent bot { model: openai }
        "#,
    );
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // Neither step's output satisfies confidence > 0; both should run.
    provider.push_response(simple_response("plain text, no JSON"));
    provider.push_response(simple_response("second result"));

    let auto_resolve = AutoResolveBlock {
        conditions: vec![AutoResolveCondition::Comparison(WhenComparison {
            field: "confidence".to_string(),
            op: CompareOp::Gt,
            value: WhenValue::Number("99".to_string()),
        })],
        span: Span::new(0, 1),
    };

    let workflow = WorkflowDef {
        name: "wf".to_string(),
        trigger: "test".to_string(),
        stages: vec![],
        steps: vec![
            make_step("first", "bot", vec![]),
            make_step("second", "bot", vec![]),
        ],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: Some(auto_resolve),
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: Span::new(0, 1),
    };
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

    let (results, events) = run_steps(&workflow, &ctx).await.expect("should succeed");
    assert_eq!(
        results.len(),
        2,
        "both steps should run when condition unmet"
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, crate::runtime::RunEvent::AutoResolved { .. })),
        "no AutoResolved event should be emitted"
    );
}

// --- #323 ---

#[tokio::test]
async fn auto_resolve_empty_conditions_does_not_short_circuit() {
    // An `auto resolve when {}` block with no conditions must NOT short-circuit.
    // Previously auto_resolve_matches returned Some("") on empty conditions, which
    // triggered an AutoResolved event and aborted the workflow after step 1.
    let file = parse_file(
        r#"
        agent bot { model: openai }
        "#,
    );
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // Use valid JSON output so auto_resolve_matches actually parses it
    // (plain text would return None early via JSON parse failure).
    provider.push_response(simple_response(r#"{"status": "done"}"#));
    provider.push_response(simple_response(r#"{"status": "done"}"#));

    let workflow = WorkflowDef {
        name: "wf".to_string(),
        trigger: "go".to_string(),
        mode: ExecutionMode::Sequential,
        stages: vec![],
        steps: vec![
            make_step("step1", "bot", vec![]),
            make_step("step2", "bot", vec![]),
        ],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: Some(AutoResolveBlock {
            conditions: vec![], // empty — must NOT short-circuit
            span: Span::new(0, 1),
        }),
        within_blocks: vec![],
        schedule: None,
        span: Span::new(0, 1),
    };

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

    let (results, events) = run_steps(&workflow, &ctx)
        .await
        .expect("workflow should succeed");

    // Both steps must run — empty conditions must not trigger early exit.
    assert_eq!(
        results.len(),
        2,
        "both steps should run when conditions is empty"
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, crate::runtime::RunEvent::AutoResolved { .. })),
        "empty conditions must not emit AutoResolved"
    );
}

// ---------------------------------------------------------------------------
// #336: run_sequential / run_parallel must propagate RunEvents
// ---------------------------------------------------------------------------

#[tokio::test]
async fn run_sequential_populates_events() {
    // #336: run_sequential must surface RunEvents from agent runs in WorkflowResult.events.
    // Each agent run emits at least one LlmCall event, so we assert both non-empty
    // and the presence of an LlmCall to pin the contract.
    let file = parse_file("agent a { model: openai }");
    let workflow = make_workflow("pipe", "hello", &["a"]);
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
    provider.push_response(simple_response("done"));

    let result = run_sequential(&workflow, &ctx)
        .await
        .expect("should succeed");

    assert!(
        result
            .events
            .iter()
            .any(|e| matches!(e, crate::runtime::RunEvent::LlmCall { .. })),
        "run_sequential events must include at least one LlmCall from the agent run trace"
    );
}

#[tokio::test]
async fn run_parallel_populates_events() {
    // #336: run_parallel must surface RunEvents from agent runs in WorkflowResult.events.
    // Two agents are run, each emitting at least one LlmCall, so we assert at least
    // two events total and the presence of an LlmCall.
    let file = parse_file(
        r"
        agent a { model: openai }
        agent b { model: openai }
    ",
    );
    let mut workflow = make_workflow("pipe", "hello", &["a", "b"]);
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
    provider.push_response(simple_response("from_a"));
    provider.push_response(simple_response("from_b"));

    let result = run_parallel(&workflow, &ctx).await.expect("should succeed");

    assert!(
        result
            .events
            .iter()
            .any(|e| matches!(e, crate::runtime::RunEvent::LlmCall { .. })),
        "run_parallel events must include at least one LlmCall from the agent run traces"
    );
    assert!(
        result.events.len() >= 2,
        "run_parallel with 2 stages must produce at least 2 events, got {}",
        result.events.len()
    );
}

// ---------------------------------------------------------------------------
// #356: StepStarted / StepCompleted events (StepFailed emission deferred to #380)
// ---------------------------------------------------------------------------

/// #356, #403: StepStarted must be emitted before StepCompleted (ordering invariant).
#[tokio::test]
async fn run_steps_emits_step_started_and_completed() {
    let file = parse_file("agent worker { model: openai }");
    let provider = MockProvider::new();
    let executor = MockExecutor::new();
    provider.push_response(simple_response("done"));

    let workflow = make_workflow_steps("wf", "go", vec![make_step("do_work", "worker", vec![])]);
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

    let (_, events) = run_steps(&workflow, &ctx).await.expect("should succeed");

    // Existence assertions.
    let started_pos = events
        .iter()
        .position(
            |e| matches!(e, crate::runtime::RunEvent::StepStarted { step, index: 0 } if step == "do_work"),
        )
        .expect("StepStarted { step: do_work, index: 0 } must be emitted");
    let completed_pos = events
        .iter()
        .position(
            |e| matches!(e, crate::runtime::RunEvent::StepCompleted { step } if step == "do_work"),
        )
        .expect("StepCompleted for do_work must be emitted");

    // #403: Ordering invariant — StepStarted must precede StepCompleted.
    assert!(
        started_pos < completed_pos,
        "StepStarted (pos {started_pos}) must precede StepCompleted (pos {completed_pos})"
    );
}

/// #404: Two-step workflow must emit StepStarted with index 0 then index 1 in order.
#[tokio::test]
async fn multi_step_step_started_index_sequence() {
    let file = parse_file("agent worker { model: openai }");
    let provider = MockProvider::new();
    let executor = MockExecutor::new();
    provider.push_response(simple_response("step-a done"));
    provider.push_response(simple_response("step-b done"));

    let step_a = make_step("step_a", "worker", vec![]);
    let step_b = make_step("step_b", "worker", vec![]);
    let workflow = make_workflow_steps("wf", "go", vec![step_a, step_b]);
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

    let (_, events) = run_steps(&workflow, &ctx).await.expect("should succeed");

    let started_events: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let crate::runtime::RunEvent::StepStarted { step, index } = e {
                Some((step.clone(), *index))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(
        started_events.len(),
        2,
        "expected 2 StepStarted events; got: {started_events:?}"
    );
    assert_eq!(
        started_events[0],
        ("step_a".to_string(), 0),
        "first step must be index 0"
    );
    assert_eq!(
        started_events[1],
        ("step_b".to_string(), 1),
        "second step must be index 1"
    );
}

/// #404 follow-up: `index` in `StepStarted` is the DAG enumeration position.
/// `resolve_dag` places independent steps (no deps) before dependent ones, so
/// step_c (independent) gets index 1 even though it appears after step_b in the
/// source file. Skipped steps (step_b) never emit `StepStarted`, so their index
/// is never visible in the event stream — but the remaining steps keep their
/// enumerate positions (no index reset).
#[tokio::test]
async fn step_started_index_reflects_dag_position_after_skip() {
    let file = parse_file("agent bot { model: openai }");
    let provider = MockProvider::new();
    let executor = MockExecutor::new();
    provider.push_response(simple_response("step-a output")); // step_a starts but fails (wrong agent)
    provider.push_response(simple_response("step-c done")); // step_c runs

    // step_a: nonexistent agent → fails.
    // step_b: depends on step_a → cascade-skipped (no StepStarted emitted).
    // step_c: independent → DAG places it at index 1 (before step_b in topo order).
    let step_a = make_step("step_a", "nonexistent", vec![]);
    let step_b = make_step("step_b", "bot", vec!["step_a"]);
    let step_c = make_step("step_c", "bot", vec![]);

    let workflow = make_workflow_steps("wf", "go", vec![step_a, step_b, step_c]);
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

    let (_, events) = run_steps(&workflow, &ctx)
        .await
        .expect("partial success — independent step still runs");

    // step_a fails (index 0), step_b is cascade-skipped (no StepStarted),
    // step_c is independent and lands at index 1 in the topological order
    // (resolve_dag places no-dep steps together, before dep-carrying steps).
    let step_c_started = events.iter().find(|e| {
        matches!(e, crate::runtime::RunEvent::StepStarted { step, index: 1 } if step == "step_c")
    });
    assert!(
        step_c_started.is_some(),
        "step_c must have StepStarted with index=1 (its topo-sort position); events: {events:?}"
    );
    // step_b must NOT emit StepStarted — it was cascade-skipped.
    assert!(
        !events.iter().any(
            |e| matches!(e, crate::runtime::RunEvent::StepStarted { step, .. } if step == "step_b")
        ),
        "step_b must not emit StepStarted (it was skipped); events: {events:?}"
    );
}

/// Tests that `run_steps` returns partial success when a step's agent is not
/// found. Under the partial-success model, `AgentNotFound` is a soft error:
/// `run_steps` returns `Ok` with a `StepFailed` event in the trace rather
/// than `Err`. This allows subsequent independent steps to continue.
#[tokio::test]
async fn run_steps_returns_partial_success_on_missing_agent() {
    let file = parse_file("agent other { model: openai }");
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    let step = make_step("broken", "ghost_agent", vec![]);
    let workflow = WorkflowDef {
        name: "wf".to_string(),
        trigger: "go".to_string(),
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

    let (results, events) = run_steps(&workflow, &ctx)
        .await
        .expect("partial success: soft error should not return Err");

    // The step result must have status Failed (not Executed or Skipped).
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].status, StageResultStatus::Failed);

    // A StepFailed event must be emitted with the agent-not-found reason.
    assert!(
        events.iter().any(|e| matches!(
            e,
            crate::runtime::RunEvent::StepFailed { step, reason, .. }
            if step == "broken" && reason.contains("ghost_agent")
        )),
        "expected StepFailed for broken; events: {events:?}"
    );
}

/// #356 — StepStarted and StepCompleted must wrap the for_each iteration set,
/// not just the regular (non-for_each) execution path.
#[tokio::test]
async fn run_steps_emits_step_started_and_completed_for_for_each() {
    let file = parse_file(r"agent bot { model: openai }");
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // Two items → two responses from the for_each loop.
    provider.push_response(simple_response("out-x"));
    provider.push_response(simple_response("out-y"));

    let step = make_step_for_each("each_step", "bot", "items");
    let workflow = WorkflowDef {
        name: "wf".to_string(),
        trigger: r#"{"items": ["x", "y"]}"#.to_string(),
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

    let (_, events) = run_steps(&workflow, &ctx).await.expect("should succeed");

    // StepStarted must be emitted before the for_each iterations.
    assert!(
        events.iter().any(|e| matches!(
            e,
            crate::runtime::RunEvent::StepStarted { step, index: 0 }
            if step == "each_step"
        )),
        "expected StepStarted {{ step: each_step, index: 0 }}"
    );

    // StepCompleted must be emitted after all iterations finish.
    assert!(
        events.iter().any(|e| matches!(
            e,
            crate::runtime::RunEvent::StepCompleted { step }
            if step == "each_step"
        )),
        "expected StepCompleted for each_step"
    );
}

// --- #363 Step Failure Skips Dependent Steps ---

fn make_workflow_steps(name: &str, trigger: &str, steps: Vec<StepDef>) -> WorkflowDef {
    WorkflowDef {
        name: name.to_string(),
        trigger: trigger.to_string(),
        stages: vec![],
        steps,
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: None,
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: Span::new(0, 1),
    }
}

/// Step A uses agent "nonexistent" (not in file) so AgentNotFound is returned.
/// Step B depends on step A and should be skipped with a StepSkipped event.
#[tokio::test]
async fn failed_dependency_skips_dependent_step() {
    let file = parse_file(r#"agent bot { model: openai }"#);

    let step_a = make_step("step_a", "nonexistent", vec![]);
    let step_b = make_step("step_b", "bot", vec!["step_a"]);

    let workflow = make_workflow_steps("dag_skip", "start", vec![step_a, step_b]);
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

    // Only step_a will run (and fail); step_b must be skipped without needing a response.
    let (results, events) = run_steps(&workflow, &ctx)
        .await
        .expect("run_steps returns Ok even when steps fail/skip");

    // step_b should appear in results as a skipped entry (empty output)
    assert_eq!(results.len(), 2, "both steps should produce a result entry");

    // StepFailed event must be emitted for step_a (the step that actually failed)
    let failed_event = events.iter().find(
        |e| matches!(e, crate::runtime::RunEvent::StepFailed { step, .. } if step == "step_a"),
    );
    assert!(
        failed_event.is_some(),
        "expected StepFailed for step_a, got events: {events:?}"
    );

    // StepSkipped event must be emitted for step_b
    let skipped_event = events.iter().find(
        |e| matches!(e, crate::runtime::RunEvent::StepSkipped { step, .. } if step == "step_b"),
    );
    assert!(
        skipped_event.is_some(),
        "expected StepSkipped for step_b, got events: {events:?}"
    );
}

/// Build a `StepDef` with the given approval gate. Avoids 24-line boilerplate
/// duplication in every approval test.
fn make_approved_step(name: &str, agent: &str, approval: crate::ast::ApprovalDef) -> StepDef {
    use crate::ast::Span as AstSpan;
    StepDef {
        name: name.to_string(),
        agent: agent.to_string(),
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
        approval: Some(approval),
        span: AstSpan::new(0, 1),
    }
}

/// CLI approval gate used across multiple approval tests.
fn make_cli_approval_def() -> crate::ast::ApprovalDef {
    use crate::ast::{ApprovalDef, ApprovalKind, Span as AstSpan};
    ApprovalDef {
        kind: ApprovalKind::Approve,
        channel: "cli".to_string(),
        destination: "#ops".to_string(),
        timeout: Some("1m".to_string()),
        mode: None,
        span: AstSpan::new(0, 1),
    }
}

/// `ApprovalTimedOut` is a hard error and must abort `run_steps` immediately
/// (not be absorbed as a soft failure).
#[tokio::test]
async fn approval_timed_out_aborts_workflow() {
    use crate::runtime::approval::ApprovalStatus;
    use std::sync::Arc;

    struct TimedOutHandler;
    #[async_trait::async_trait]
    impl crate::runtime::approval::ApprovalHandler for TimedOutHandler {
        async fn request_approval(
            &self,
            _step: &str,
            _output: &str,
            _approval: &crate::ast::ApprovalDef,
        ) -> ApprovalStatus {
            ApprovalStatus::TimedOut
        }
    }

    let file = parse_file(r#"agent bot { model: openai }"#);
    let step_a = make_approved_step("gated", "bot", make_cli_approval_def());
    let workflow = make_workflow_steps("timed_out_wf", "start", vec![step_a]);

    let provider = MockProvider::new();
    let executor = MockExecutor::new();
    provider.push_response(simple_response("output"));

    let ctx = WorkflowContext {
        file: &file,
        provider: &provider,
        executor: &executor,
        tool_defs: &[],
        config: &RunConfig::default(),
        approval_handler: Some(Arc::new(TimedOutHandler)),
        audit_log: None,
        workflow_name: None,
    };

    let result = run_steps(&workflow, &ctx).await;
    assert!(
        matches!(result, Err(WorkflowError::ApprovalTimedOut { .. })),
        "ApprovalTimedOut must abort run_steps immediately; got: {result:?}"
    );
}

/// `ApprovalRejected` is a hard error and must abort `run_steps` immediately
/// (not be absorbed as a soft failure). Mirrors `approval_timed_out_aborts_workflow`.
#[tokio::test]
async fn approval_rejected_aborts_workflow() {
    use crate::runtime::approval::ApprovalStatus;
    use std::sync::Arc;

    struct RejectHandler;
    #[async_trait::async_trait]
    impl crate::runtime::approval::ApprovalHandler for RejectHandler {
        async fn request_approval(
            &self,
            _step: &str,
            _output: &str,
            _approval: &crate::ast::ApprovalDef,
        ) -> ApprovalStatus {
            ApprovalStatus::Rejected {
                reason: "policy violation".to_string(),
            }
        }
    }

    let file = parse_file(r#"agent bot { model: openai }"#);
    let step_a = make_approved_step("gated", "bot", make_cli_approval_def());
    let workflow = make_workflow_steps("rejected_wf", "start", vec![step_a]);

    let provider = MockProvider::new();
    let executor = MockExecutor::new();
    provider.push_response(simple_response("output"));

    let ctx = WorkflowContext {
        file: &file,
        provider: &provider,
        executor: &executor,
        tool_defs: &[],
        config: &RunConfig::default(),
        approval_handler: Some(Arc::new(RejectHandler)),
        audit_log: None,
        workflow_name: None,
    };

    let result = run_steps(&workflow, &ctx).await;
    assert!(
        matches!(result, Err(WorkflowError::ApprovalRejected { .. })),
        "ApprovalRejected must abort run_steps immediately; got: {result:?}"
    );
}

/// `ApprovalStatus::Pending` (deferred / async approval) must return
/// `WorkflowError::ApprovalPending`, not `ApprovalTimedOut`. (#419)
#[tokio::test]
async fn approval_pending_returns_approval_pending_error() {
    use crate::runtime::approval::ApprovalStatus;
    use std::sync::Arc;

    struct PendingHandler;
    #[async_trait::async_trait]
    impl crate::runtime::approval::ApprovalHandler for PendingHandler {
        async fn request_approval(
            &self,
            _step: &str,
            _output: &str,
            _approval: &crate::ast::ApprovalDef,
        ) -> ApprovalStatus {
            ApprovalStatus::Pending
        }
    }

    let file = parse_file(r#"agent bot { model: openai }"#);
    // `make_cli_approval_def()` is used for convenience — the channel type is
    // irrelevant here because `PendingHandler` overrides the handler entirely.
    // The test exercises the workflow engine's response to `ApprovalStatus::Pending`,
    // not the CLI handler's interactive flow.
    let step_a = make_approved_step("gated", "bot", make_cli_approval_def());
    let workflow = make_workflow_steps("pending_wf", "start", vec![step_a]);

    let provider = MockProvider::new();
    let executor = MockExecutor::new();
    provider.push_response(simple_response("output"));

    let ctx = WorkflowContext {
        file: &file,
        provider: &provider,
        executor: &executor,
        tool_defs: &[],
        config: &RunConfig::default(),
        approval_handler: Some(Arc::new(PendingHandler)),
        audit_log: None,
        workflow_name: None,
    };

    let result = run_steps(&workflow, &ctx).await;
    assert!(
        matches!(result, Err(WorkflowError::ApprovalPending { .. })),
        "Pending approval must return ApprovalPending, not ApprovalTimedOut; got: {result:?}"
    );
}

/// Steps with no dependency on the failed step should still execute.
#[tokio::test]
async fn independent_step_runs_even_if_sibling_fails() {
    let file = parse_file(r#"agent bot { model: openai }"#);

    // step_a: fails (nonexistent agent)
    // step_b: independent (no depends_on) — should still run
    let step_a = make_step("step_a", "nonexistent", vec![]);
    let step_b = make_step("step_b", "bot", vec![]);

    let workflow = make_workflow_steps("dag_sibling", "start", vec![step_a, step_b]);
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

    // step_b (bot) runs successfully
    provider.push_response(simple_response("step_b_output"));

    let (results, events) = run_steps(&workflow, &ctx)
        .await
        .expect("run_steps must not abort when only independent step fails");

    // step_a must be recorded as Failed (not Executed or Skipped)
    let step_a_result = results
        .iter()
        .find(|r| r.stage_name == "step_a")
        .expect("step_a must have a result entry");
    assert_eq!(
        step_a_result.status,
        StageResultStatus::Failed,
        "failed step must have status Failed; results: {results:?}"
    );
    // A StepFailed event must be emitted — this is the trigger condition for the scenario.
    assert!(
        events.iter().any(
            |e| matches!(e, crate::runtime::RunEvent::StepFailed { step, .. } if step == "step_a")
        ),
        "expected StepFailed event for step_a; events: {events:?}"
    );
    // step_b should have produced output
    let step_b_result = results
        .iter()
        .find(|r| r.stage_name == "step_b")
        .expect("step_b should have a result entry");
    assert_eq!(
        step_b_result.output, "step_b_output",
        "step_b output mismatch; results: {results:?}"
    );
}

/// #363 — Skipped StageResult has `status == StageResultStatus::Skipped` so
/// consumers can distinguish it from a successfully-run step.
#[tokio::test]
async fn skipped_step_result_uses_skipped_status() {
    let file = parse_file(r#"agent bot { model: openai }"#);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // step_a: fails; step_b: depends on step_a → skipped.
    let step_a = make_step("step_a", "nonexistent", vec![]);
    let step_b = make_step("step_b", "bot", vec!["step_a"]);
    let workflow = make_workflow_steps("status_test", "start", vec![step_a, step_b]);
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

    let (results, _events) = run_steps(&workflow, &ctx)
        .await
        .expect("soft error must not abort run");

    let step_b_result = results.iter().find(|r| r.stage_name == "step_b").unwrap();
    assert_eq!(
        step_b_result.status,
        StageResultStatus::Skipped,
        "skipped StageResult must have status Skipped"
    );
}

/// #363 — A step whose dependency failed still sees an (empty) entry in the
/// `outputs` map. Downstream steps using `filter_map` must not silently drop
/// the gap; it is observable via the empty string, not a missing key.
#[tokio::test]
async fn failed_step_output_inserted_into_outputs_map() {
    let file = parse_file(r#"agent bot { model: openai }"#);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // step_a (fails) → step_b (depends on step_a, skipped).
    // step_c is independent and should run normally.
    let step_a = make_step("step_a", "nonexistent", vec![]);
    let step_b = make_step("step_b", "bot", vec!["step_a"]);
    let step_c = make_step("step_c", "bot", vec![]);
    let workflow = make_workflow_steps("outputs_gap", "start", vec![step_a, step_b, step_c]);
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

    provider.push_response(simple_response("step_c_out"));

    let (results, events) = run_steps(&workflow, &ctx)
        .await
        .expect("independent step must run");

    // step_c ran and produced output
    let step_c_r = results.iter().find(|r| r.stage_name == "step_c").unwrap();
    assert_eq!(step_c_r.output, "step_c_out");

    // step_a failed → StepFailed event; step_b skipped → StepSkipped event
    assert!(events.iter().any(
        |e| matches!(e, crate::runtime::RunEvent::StepFailed { step, .. } if step == "step_a")
    ));
    assert!(events.iter().any(
        |e| matches!(e, crate::runtime::RunEvent::StepSkipped { step, .. } if step == "step_b")
    ));

    // Failed and skipped results carry the correct status
    let step_a_r = results.iter().find(|r| r.stage_name == "step_a").unwrap();
    assert_eq!(step_a_r.status, StageResultStatus::Failed);
    let step_b_r = results.iter().find(|r| r.stage_name == "step_b").unwrap();
    assert_eq!(step_b_r.status, StageResultStatus::Skipped);
}

/// `CyclicDependency` is a hard error — `run_steps` must return `Err` and
/// not attempt to execute any step when the dependency graph has a cycle.
#[tokio::test]
async fn cyclic_dependency_is_hard_error() {
    let file = parse_file(r#"agent bot { model: openai }"#);

    // step_a → step_b → step_a (cycle)
    let step_a = make_step("step_a", "bot", vec!["step_b"]);
    let step_b = make_step("step_b", "bot", vec!["step_a"]);

    let workflow = make_workflow_steps("cyclic", "start", vec![step_a, step_b]);
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

    let err = run_steps(&workflow, &ctx)
        .await
        .expect_err("cyclic dependency must return Err");

    assert!(
        matches!(
            err,
            crate::runtime::workflow::WorkflowError::CyclicDependency(_)
        ),
        "expected CyclicDependency hard error, got: {err:?}"
    );
    assert!(
        err.is_hard_error(),
        "CyclicDependency must be classified as a hard error"
    );
}

/// #363/#374 — A `for_each` step that fails (agent not found) is a soft error:
/// it is recorded as `StepFailed` and its declared dependents receive
/// `StepSkipped`. The `StepSkipped` event must include `blocked_dependency`
/// set to the failing step's name (not a generic "unknown").
#[tokio::test]
async fn for_each_step_failure_cascades_to_dependent() {
    // "ghost" does not exist in the file → for_each step fails with AgentNotFound.
    let file = parse_file(r#"agent follower { model: openai }"#);
    let provider = MockProvider::new();
    // step_a uses "ghost" (AgentNotFound) — provider is never called for it.
    // step_b is skipped (depends on step_a). Only step_c (independent) runs.
    provider.push_response(simple_response("follower ran")); // consumed by step_c
    let executor = MockExecutor::new();

    // step_a: for_each with non-existent agent → will fail
    let step_a = make_step_for_each("step_a", "ghost", "items");
    // step_b: depends on step_a → should be skipped when step_a fails
    let step_b = make_step("step_b", "follower", vec!["step_a"]);
    // step_c: independent — should still run despite step_a failure
    let step_c = make_step("step_c", "follower", vec![]);

    let workflow = make_workflow_steps("cascade_test", "start", vec![step_a, step_b, step_c]);
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

    let result = run_workflow(&workflow, &ctx)
        .await
        .expect("workflow should succeed (soft error, partial success)");

    // step_a failed → StepFailed event must be emitted.
    assert!(
        result.events.iter().any(
            |e| matches!(e, crate::runtime::RunEvent::StepFailed { step, .. } if step == "step_a")
        ),
        "expected StepFailed for step_a; events: {:?}",
        result.events
    );

    // step_b skipped because step_a failed → StepSkipped with correct blocked_dependency.
    let skipped_b = result.events.iter().find(
        |e| matches!(e, crate::runtime::RunEvent::StepSkipped { step, .. } if step == "step_b"),
    );
    assert!(
        skipped_b.is_some(),
        "expected StepSkipped for step_b; events: {:?}",
        result.events
    );
    if let Some(crate::runtime::RunEvent::StepSkipped {
        blocked_dependency, ..
    }) = skipped_b
    {
        assert_eq!(
            blocked_dependency, "step_a",
            "StepSkipped.blocked_dependency must be 'step_a'"
        );
    }

    // step_c is independent and should have run with the expected output.
    assert!(
        result
            .events
            .iter()
            .any(|e| matches!(e, crate::runtime::RunEvent::StepCompleted { step, .. } if step == "step_c")),
        "step_c should have completed; events: {:?}",
        result.events
    );
    let step_c_result = result
        .stage_results
        .iter()
        .find(|r| r.stage_name == "step_c")
        .expect("step_c must have a result entry");
    assert_eq!(
        step_c_result.output, "follower ran",
        "step_c output mismatch"
    );
}

/// #363/#374 — Partial `for_each` failure: iteration 0 succeeds, iteration 1
/// fails (provider queue empty → `ProviderError::Api`). The whole step must be
/// recorded as `StepFailed` and partial results from iteration 0 must be
/// discarded. All-or-nothing semantics are required by the `for_each` contract.
#[tokio::test]
async fn for_each_partial_failure_discards_completed_iterations() {
    let file = parse_file(r#"agent bot { model: openai }"#);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // Push only one response: iteration 0 succeeds, iteration 1 fails
    // because the provider queue is empty (returns ProviderError::Api 500).
    provider.push_response(simple_response("iter0_output"));

    let step = make_step_for_each("batch", "bot", "items");
    let workflow = WorkflowDef {
        name: "wf".to_string(),
        trigger: r#"{"items": ["item0", "item1"]}"#.to_string(),
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

    // run_steps returns Ok (soft error), but the step is recorded as failed.
    let (results, events) = run_steps(&workflow, &ctx)
        .await
        .expect("soft error must not abort run_steps");

    // The step must be marked as failed, not completed.
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].status,
        StageResultStatus::Failed,
        "partial for_each failure must have status Failed"
    );
    // Partial output from iteration 0 must be discarded.
    assert!(
        results[0].output.is_empty(),
        "partial for_each results must be discarded; got: {:?}",
        results[0].output
    );
    // A StepFailed event must be emitted with a non-empty reason and error_kind
    // "stage_failed". The for_each path wraps iteration errors as
    // WorkflowError::StageFailed before calling apply_step_result.
    assert!(
        events.iter().any(|e| matches!(
            e,
            crate::runtime::RunEvent::StepFailed { step, reason, .. }
            if step == "batch" && !reason.is_empty()
        )),
        "expected StepFailed for 'batch' with non-empty reason; events: {events:?}"
    );
    assert!(
        events.iter().any(|e| matches!(
            e,
            crate::runtime::RunEvent::StepFailed { step, error_kind, .. }
            if step == "batch" && error_kind == "stage_failed"
        )),
        "for_each failure must produce error_kind \"stage_failed\"; events: {events:?}"
    );
    // No StepCompleted event for the step (it did not complete).
    assert!(
        !events.iter().any(
            |e| matches!(e, crate::runtime::RunEvent::StepCompleted { step } if step == "batch")
        ),
        "StepCompleted must not be emitted when for_each fails; events: {events:?}"
    );
}

/// #363 — When ALL steps in a workflow fail, `final_output` must be an empty
/// string. Callers (e.g. the CLI) must handle this case explicitly rather than
/// treating it as a normal completion with empty output.
#[tokio::test]
async fn all_steps_fail_gives_empty_final_output() {
    // No valid agents in the file → every step fails with AgentNotFound.
    let file = parse_file(r#"agent other { model: openai }"#);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    let step_a = make_step("step_a", "ghost_a", vec![]);
    let step_b = make_step("step_b", "ghost_b", vec![]);
    let workflow = make_workflow_steps("all_fail", "start", vec![step_a, step_b]);

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

    let result = run_workflow(&workflow, &ctx)
        .await
        .expect("partial success must not return Err");

    assert!(
        result.final_output.is_empty(),
        "final_output must be empty when all steps fail; got: {:?}",
        result.final_output
    );
    // Both steps should have failed entries in stage_results.
    assert_eq!(result.stage_results.len(), 2);
    assert!(result.stage_results.iter().all(|r| !r.is_real_execution()));
}

/// #363 — A 3-hop cascade: step_a fails → step_b (depends on step_a) is skipped
/// → step_c (depends on step_b) must ALSO be skipped. This validates that a
/// skipped step is added to `blocked_steps` so its own dependents propagate the
/// skip correctly, not just direct dependents of the original failure.
#[tokio::test]
async fn cascade_skip_propagates_three_hops() {
    let file = parse_file(r#"agent bot { model: openai }"#);
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    // step_a: fails (agent not found)
    // step_b: depends on step_a → skipped
    // step_c: depends on step_b → must also be skipped (not on step_a directly)
    let step_a = make_step("step_a", "nonexistent", vec![]);
    let step_b = make_step("step_b", "bot", vec!["step_a"]);
    let step_c = make_step("step_c", "bot", vec!["step_b"]);
    let workflow = make_workflow_steps("three_hop", "start", vec![step_a, step_b, step_c]);

    // No provider responses needed — no step should actually run.
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

    let (results, events) = run_steps(&workflow, &ctx)
        .await
        .expect("run_steps must not abort on soft errors");

    assert_eq!(
        results.len(),
        3,
        "all three steps must produce a result entry"
    );

    // step_a failed
    let step_a_r = results.iter().find(|r| r.stage_name == "step_a").unwrap();
    assert_eq!(step_a_r.status, StageResultStatus::Failed);

    // step_b skipped (direct dependent of failed step)
    let step_b_r = results.iter().find(|r| r.stage_name == "step_b").unwrap();
    assert_eq!(
        step_b_r.status,
        StageResultStatus::Skipped,
        "step_b must be skipped"
    );

    // step_c skipped (transitive — depends on skipped step_b, not on step_a)
    let step_c_r = results.iter().find(|r| r.stage_name == "step_c").unwrap();
    assert_eq!(
        step_c_r.status,
        StageResultStatus::Skipped,
        "step_c must be skipped transitively"
    );

    // StepSkipped events for both step_b and step_c
    assert!(
        events.iter().any(
            |e| matches!(e, crate::runtime::RunEvent::StepSkipped { step, .. } if step == "step_b")
        ),
        "expected StepSkipped for step_b; events: {events:?}"
    );
    assert!(
        events.iter().any(
            |e| matches!(e, crate::runtime::RunEvent::StepSkipped { step, .. } if step == "step_c")
        ),
        "expected StepSkipped for step_c (transitive); events: {events:?}"
    );
}

// #427: RunError::Timeout inside a workflow stage must propagate as a hard error
// (WorkflowError::StageTimedOut) that aborts the workflow immediately, rather
// than being treated as a soft StageFailed that would allow subsequent stages to run.
#[tokio::test(start_paused = true)]
async fn stage_timeout_in_workflow_is_hard_error() {
    use crate::runtime::provider::Message;
    use crate::runtime::provider::Provider;
    use crate::runtime::provider::ProviderError;
    use crate::runtime::provider::ToolDef;

    struct HangingProvider;

    #[async_trait::async_trait]
    impl Provider for HangingProvider {
        fn name(&self) -> &'static str {
            "hanging"
        }
        async fn chat(
            &self,
            _messages: &[Message],
            _tools: &[ToolDef],
        ) -> Result<ChatResponse, ProviderError> {
            std::future::pending().await
        }
    }

    let source = r#"
        agent slow { model: openai }
        agent fast { model: openai }
    "#;
    let file = parse_file(source);
    let workflow = make_workflow("pipe", "go", &["slow", "fast"]);
    let executor = MockExecutor::new();
    let provider = HangingProvider;
    let ctx = WorkflowContext {
        file: &file,
        provider: &provider,
        executor: &executor,
        tool_defs: &[],
        config: &RunConfig {
            stage_timeout_secs: Some(5),
            ..RunConfig::default()
        },
        approval_handler: None,
        audit_log: None,
        workflow_name: None,
    };

    let result = run_sequential(&workflow, &ctx).await;

    // Must be an error (not Ok) — the timeout must abort the workflow.
    assert!(
        result.is_err(),
        "expected error from timed-out stage; got Ok"
    );
    let err = result.unwrap_err();

    // Must be a hard error: StageTimedOut, NOT StageFailed.
    assert!(
        matches!(err, WorkflowError::StageTimedOut { .. }),
        "timeout inside workflow stage must produce StageTimedOut (hard error); got: {err:?}"
    );
}

// --- #453: final_output contract in mixed stage+step workflows ---

/// #453: When a workflow has both stages (that succeed) and steps (that all fail),
/// `final_output` must retain the last successful stage output.
///
/// This covers the mixed-workflow case documented on `WorkflowResult::final_output`:
/// "If all steps fail or are skipped, `final_output` retains the last successful
/// stage output."
#[tokio::test]
async fn mixed_workflow_final_output_retains_stage_output_when_all_steps_fail() {
    // The file defines only the stage agent; the step references "ghost_agent"
    // which is not defined — causing AgentNotFound (soft error) on the step.
    let file = parse_file(r#"agent writer { model: openai }"#);

    let workflow = WorkflowDef {
        name: "mixed_wf".to_string(),
        trigger: "start".to_string(),
        stages: vec![Stage {
            name: "writer".to_string(),
            agent: "writer".to_string(),
            route: RouteRule::Next,
            span: Span::new(0, 1),
        }],
        steps: vec![StepDef {
            name: "failing_step".to_string(),
            // "ghost_agent" is not in the file → AgentNotFound → soft fail
            agent: "ghost_agent".to_string(),
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
            approval: None,
            span: crate::ast::Span::new(0, 1),
        }],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: None,
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: Span::new(0, 1),
    };

    let provider = MockProvider::new();
    provider.push_response(simple_response("stage output"));
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

    // run_workflow must succeed even though the step fails (soft error).
    let result = run_workflow(&workflow, &ctx)
        .await
        .expect("workflow must succeed even when all steps fail");

    // final_output must be the stage output, not an empty string from the
    // failed step (which has is_real_execution() == false).
    assert_eq!(
        result.final_output, "stage output",
        "final_output must retain stage output when all steps fail; got: {:?}",
        result.final_output
    );
    // Confirm the step actually ran and was recorded as Failed, not silently skipped.
    // Without this check the test could pass vacuously if routing bypassed run_steps.
    assert!(
        result
            .stage_results
            .iter()
            .any(|r| r.status == StageResultStatus::Failed),
        "at least one step result must be Failed; got: {:?}",
        result.stage_results
    );
}

// --- #452: error_kind field on StepFailed ---

/// #452: StepFailed must carry an `error_kind` field set to the snake_case
/// WorkflowError variant name (e.g. "agent_not_found", "stage_failed").
#[tokio::test]
async fn step_failed_carries_error_kind_agent_not_found() {
    let file = parse_file("agent other { model: openai }");
    let provider = MockProvider::new();
    let executor = MockExecutor::new();

    let workflow = WorkflowDef {
        name: "wf".to_string(),
        trigger: "go".to_string(),
        stages: vec![],
        steps: vec![make_step("broken", "nonexistent_agent", vec![])],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: None,
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: Span::new(0, 1),
    };
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

    let (_, events) = run_steps(&workflow, &ctx).await.unwrap();
    let failed = events
        .iter()
        .find(
            |e| matches!(e, crate::runtime::RunEvent::StepFailed { step, .. } if step == "broken"),
        )
        .expect("StepFailed for 'broken' must be emitted");

    let crate::runtime::RunEvent::StepFailed { error_kind, .. } = failed else {
        panic!("expected StepFailed variant");
    };
    assert_eq!(
        error_kind, "agent_not_found",
        "AgentNotFound error must produce error_kind=\"agent_not_found\""
    );
}

/// #452: StepFailed for a stage execution failure must carry error_kind "stage_failed".
#[tokio::test]
async fn step_failed_carries_error_kind_stage_failed() {
    let file = parse_file("agent worker { model: openai }");
    // MockProvider::push_error returns a provider error string. That maps to
    // WorkflowError::StageFailed (the soft-error path), confirmed by the existing
    // provider_error_produces_stage_failed test. It is NOT a timeout, so it does not
    // produce WorkflowError::StageTimedOut (hard error). This is the source of the
    // expected error_kind "stage_failed".
    let provider = MockProvider::new();
    provider.push_error("simulated network failure");
    let executor = MockExecutor::new();

    let workflow = WorkflowDef {
        name: "wf".to_string(),
        trigger: "go".to_string(),
        stages: vec![],
        steps: vec![make_step("do_work", "worker", vec![])],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: None,
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: Span::new(0, 1),
    };
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

    let (_, events) = run_steps(&workflow, &ctx).await.unwrap();
    let failed = events
        .iter()
        .find(
            |e| matches!(e, crate::runtime::RunEvent::StepFailed { step, .. } if step == "do_work"),
        )
        .expect("StepFailed for 'do_work' must be emitted");

    let crate::runtime::RunEvent::StepFailed { error_kind, .. } = failed else {
        panic!("expected StepFailed variant");
    };
    assert_eq!(
        error_kind, "stage_failed",
        "StageFailed error must produce error_kind=\"stage_failed\""
    );
}
