use super::*;
use crate::ast::{ConditionMatcher, ExecutionMode, RouteRule, Span, Stage, WorkflowDef};
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
    };

    // Queue response for agent "a" so it doesn't fail first
    provider.push_response(simple_response("ok"));

    let err = run_parallel(&workflow, &ctx).await.unwrap_err();

    assert!(err.to_string().contains("missing"), "err: {err}");
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
    };

    let result = run_workflow(&workflow, &ctx).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("rejected"), "error should mention rejection: {err}");
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
    };

    provider.push_response(simple_response("Done without approval"));

    let result = run_workflow(&workflow, &ctx).await.unwrap();
    assert_eq!(result.final_output, "Done without approval");
}
