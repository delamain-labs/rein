use std::sync::Arc;

use crate::ast::{ExecutionMode, ReinFile, RouteRule, Stage, WorkflowDef};

/// Sentinel agent name used in `StageResult` for steps that failed softly.
/// Consumers of `WorkflowResult.stage_results` can use this to distinguish
/// failed steps from steps that actually ran.
pub const SENTINEL_FAILED: &str = "<failed>";
/// Sentinel agent name used in `StageResult` for steps skipped due to a
/// failed dependency. Consumers of `WorkflowResult.stage_results` can use
/// this to distinguish skipped steps from steps that actually ran.
pub const SENTINEL_SKIPPED: &str = "<skipped>";
use crate::runtime::approval::{ApprovalHandler, ApprovalStatus};

use super::engine::{AgentEngine, RunConfig};
use super::executor::ToolExecutor;
use super::permissions::ToolRegistry;
use super::provider::{Provider, ToolDef};

mod condition;
pub mod persistence;
mod resumable;

use condition::condition_matches;
pub use resumable::run_sequential_resumable;

#[cfg(test)]
mod tests;

/// Bundles the shared dependencies needed to execute workflow stages.
///
/// Eliminates repetitive parameter lists across `run_sequential`,
/// `run_parallel`, `run_sequential_resumable`, and `run_workflow`.
pub struct WorkflowContext<'a> {
    pub file: &'a ReinFile,
    pub provider: &'a dyn Provider,
    pub executor: &'a dyn ToolExecutor,
    pub tool_defs: &'a [ToolDef],
    pub config: &'a RunConfig,
    pub approval_handler: Option<Arc<dyn ApprovalHandler>>,
}

/// The result of a completed workflow run.
#[derive(Debug)]
pub struct WorkflowResult {
    /// Results from each stage, in order of execution.
    pub stage_results: Vec<StageResult>,
    /// The final output text.
    pub final_output: String,
    /// Total cost across all stages.
    pub total_cost_cents: u64,
    /// Total tokens across all stages.
    pub total_tokens: u64,
    /// All `RunEvent`s collected during the workflow run. For stage-based
    /// executions (sequential/parallel) this includes agent-level events
    /// from every stage: `LlmCall`, `ToolCallAttempt`, `BudgetUpdate`, etc.
    /// For step-based executions this contains workflow-level events:
    /// `StepStarted`, `StepCompleted`, `StepFailed`, `StepSkipped`,
    /// `StepFallback`, `ForEachIteration`, `AutoResolved`.
    pub events: Vec<super::RunEvent>,
}

/// The result of a single stage within a workflow.
#[derive(Debug)]
pub struct StageResult {
    pub stage_name: String,
    pub agent_name: String,
    pub output: String,
    pub cost_cents: u64,
    pub tokens: u64,
}

/// Errors that can occur during workflow execution.
#[derive(Debug)]
pub enum WorkflowError {
    /// A stage's agent was not found in the file.
    AgentNotFound(String),
    /// A stage failed during execution.
    StageFailed {
        stage: String,
        error: super::RunError,
    },
    /// A route references a stage that doesn't exist.
    StageNotFound(String),
    /// State persistence failed (save/load/clear).
    PersistenceFailure(String),
    /// A circular route was detected.
    CircularRoute(String),
    /// A step's approval gate was rejected.
    ApprovalRejected { step: String, reason: String },
    /// A step's approval gate timed out.
    ApprovalTimedOut { step: String },
    /// A cyclic dependency was detected in step `depends_on` declarations.
    CyclicDependency(String),
}

impl std::fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AgentNotFound(name) => write!(f, "agent not found: {name}"),
            Self::StageFailed { stage, error } => {
                write!(f, "stage '{stage}' failed: {error}")
            }
            Self::StageNotFound(name) => write!(f, "route target stage not found: {name}"),
            Self::PersistenceFailure(msg) => write!(f, "state persistence failed: {msg}"),
            Self::CircularRoute(name) => write!(f, "circular route detected at stage '{name}'"),
            Self::ApprovalRejected { step, reason } => {
                write!(f, "approval rejected for step '{step}': {reason}")
            }
            Self::ApprovalTimedOut { step } => {
                write!(f, "approval timed out for step '{step}'")
            }
            Self::CyclicDependency(detail) => {
                write!(f, "Cycle detected in workflow step dependencies: {detail}")
            }
        }
    }
}

impl WorkflowError {
    /// Returns `true` for errors that must abort the entire workflow immediately
    /// (policy enforcement, infrastructure invariants).
    ///
    /// Returns `false` for soft errors (agent not found, LLM failure) where the
    /// step should be recorded as failed and the workflow should continue,
    /// skipping any steps that depend on the failed step.
    ///
    /// Classification uses an exhaustive `match` (no wildcard) so that adding a
    /// new `WorkflowError` variant produces a compile error until its hard/soft
    /// status is made explicit.
    ///
    /// Hard errors: policy enforcement, topology bugs, infrastructure failures.
    /// Soft errors: transient agent/LLM failures where the step is skipped and
    /// dependent steps are cascaded; the workflow continues.
    #[must_use]
    pub fn is_hard_error(&self) -> bool {
        match self {
            // Policy enforcement — workflow must not continue after a rejected
            // or timed-out human approval gate. Topology/config bugs (cyclic
            // deps, circular routes, missing route targets) mean the graph
            // itself is malformed — silently absorbing them hides operator
            // misconfiguration. State persistence failure risks data corruption.
            Self::ApprovalRejected { .. }
            | Self::ApprovalTimedOut { .. }
            | Self::CyclicDependency(_)
            | Self::CircularRoute(_)
            | Self::PersistenceFailure(_)
            | Self::StageNotFound(_) => true,
            // Soft — step is recorded as failed; dependents are skipped.
            Self::AgentNotFound(_) | Self::StageFailed { .. } => false,
        }
    }
}

impl std::error::Error for WorkflowError {}

/// Run a single stage and return its result.
pub(super) async fn run_stage(
    stage_name: &str,
    agent_name: &str,
    input: &str,
    ctx: &WorkflowContext<'_>,
) -> Result<(StageResult, Vec<super::RunEvent>), WorkflowError> {
    let agent = ctx
        .file
        .agents
        .iter()
        .find(|a| a.name == agent_name)
        .ok_or_else(|| WorkflowError::AgentNotFound(agent_name.to_string()))?;

    let registry = ToolRegistry::from_agent(agent);
    let engine = AgentEngine::new(
        ctx.provider,
        ctx.executor,
        &registry,
        ctx.tool_defs.to_vec(),
        ctx.config.clone(),
    );

    let result = engine
        .run(input)
        .await
        .map_err(|e| WorkflowError::StageFailed {
            stage: stage_name.to_string(),
            error: e,
        })?;

    let events = result.trace.events;
    Ok((
        StageResult {
            stage_name: stage_name.to_string(),
            agent_name: agent_name.to_string(),
            output: result.response,
            cost_cents: result.total_cost_cents,
            tokens: result.total_tokens,
        },
        events,
    ))
}

/// Collect stage results into a `WorkflowResult`.
pub(super) fn build_result(
    stage_results: Vec<StageResult>,
    final_output: String,
    events: Vec<super::RunEvent>,
) -> WorkflowResult {
    let total_cost = stage_results.iter().map(|r| r.cost_cents).sum();
    let total_tokens = stage_results.iter().map(|r| r.tokens).sum();
    WorkflowResult {
        stage_results,
        final_output,
        total_cost_cents: total_cost,
        total_tokens,
        events,
    }
}

/// Resolve the next stage given the current stage's route rule and output.
///
/// This is the single source of truth for routing logic, used by both
/// `run_sequential` and `run_sequential_resumable`.
pub(super) fn resolve_next_stage<'a>(
    workflow: &'a WorkflowDef,
    current: &Stage,
    output: &str,
) -> Result<Option<&'a Stage>, WorkflowError> {
    match &current.route {
        RouteRule::Next => {
            let idx = workflow.stages.iter().position(|s| s.name == current.name);
            Ok(idx.and_then(|i| workflow.stages.get(i + 1)))
        }
        RouteRule::Conditional {
            field,
            matcher,
            then_stage,
            else_stage,
        } => {
            if condition_matches(output, field, matcher) {
                Ok(Some(workflow.find_stage(then_stage).ok_or_else(|| {
                    WorkflowError::StageNotFound(then_stage.clone())
                })?))
            } else if let Some(else_name) = else_stage {
                Ok(Some(workflow.find_stage(else_name).ok_or_else(|| {
                    WorkflowError::StageNotFound(else_name.clone())
                })?))
            } else {
                Ok(None)
            }
        }
    }
}

/// Execute a workflow sequentially: each stage's output becomes the next
/// stage's input. Respects [`RouteRule::Conditional`] for branching to named
/// stages.
///
/// # Errors
/// Returns `WorkflowError` if an agent is missing, a stage fails, a route
/// references a nonexistent stage, or a circular route is detected.
pub async fn run_sequential(
    workflow: &WorkflowDef,
    ctx: &WorkflowContext<'_>,
) -> Result<WorkflowResult, WorkflowError> {
    let mut stage_results = Vec::new();
    let mut all_events: Vec<super::RunEvent> = Vec::new();
    let mut current_input = format!("Trigger: {}", workflow.trigger);
    let mut visited = std::collections::HashSet::new();

    let mut current_stage: Option<&Stage> = workflow.stages.first();

    while let Some(stage) = current_stage {
        if !visited.insert(&*stage.name) {
            return Err(WorkflowError::CircularRoute(stage.name.clone()));
        }

        let (result, stage_events) =
            run_stage(&stage.name, &stage.agent, &current_input, ctx).await?;

        current_input.clone_from(&result.output);
        let output = result.output.clone();
        stage_results.push(result);
        all_events.extend(stage_events);

        current_stage = resolve_next_stage(workflow, stage, &output)?;
    }

    let final_output = stage_results
        .last()
        .map(|r| r.output.clone())
        .unwrap_or_default();

    Ok(build_result(stage_results, final_output, all_events))
}

/// Execute all workflow stages with the trigger as input (fan-out pattern).
///
/// Stages run **concurrently** — each receives the same trigger input
/// independently and all are polled simultaneously. Results are returned in
/// stage-declaration order regardless of completion order. The first stage
/// error short-circuits and returns immediately.
///
/// # Errors
/// Returns `WorkflowError` if an agent is missing or any stage fails.
pub async fn run_parallel(
    workflow: &WorkflowDef,
    ctx: &WorkflowContext<'_>,
) -> Result<WorkflowResult, WorkflowError> {
    use futures::future::try_join_all;

    let trigger_input = format!("Trigger: {}", workflow.trigger);

    let outcomes = try_join_all(
        workflow
            .stages
            .iter()
            .map(|stage| run_stage(&stage.name, &stage.agent, &trigger_input, ctx)),
    )
    .await?;

    let mut stage_results = Vec::with_capacity(outcomes.len());
    let mut all_events: Vec<super::RunEvent> = Vec::new();
    for (result, stage_events) in outcomes {
        stage_results.push(result);
        all_events.extend(stage_events);
    }

    let final_output = stage_results
        .iter()
        .map(|r| format!("[{}]: {}", r.stage_name, r.output))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(build_result(stage_results, final_output, all_events))
}

/// Resolve step execution order using Kahn's topological sort algorithm.
///
/// Steps with no `depends_on` maintain their relative file order.
/// Returns `Err(WorkflowError::CyclicDependency)` if a cycle is detected.
pub fn resolve_dag(
    steps: &[crate::ast::StepDef],
) -> Result<Vec<&crate::ast::StepDef>, WorkflowError> {
    use std::collections::HashMap;

    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

    for step in steps {
        in_degree.entry(&step.name).or_insert(0);
        for dep in &step.depends_on {
            *in_degree.entry(&step.name).or_insert(0) += 1;
            dependents.entry(dep.as_str()).or_default().push(&step.name);
        }
    }

    let mut ready: Vec<&str> = in_degree
        .iter()
        .filter(|&(_, &d)| d == 0)
        .map(|(&name, _)| name)
        .collect();

    // Stable sort: steps with no deps preserve file order
    let step_index: HashMap<&str, usize> = steps
        .iter()
        .enumerate()
        .map(|(i, s)| (s.name.as_str(), i))
        .collect();
    ready.sort_by_key(|name| step_index.get(name).copied().unwrap_or(usize::MAX));

    let step_by_name: HashMap<&str, &crate::ast::StepDef> =
        steps.iter().map(|s| (s.name.as_str(), s)).collect();

    let mut order: Vec<&crate::ast::StepDef> = Vec::new();
    let mut i = 0;

    while i < ready.len() {
        let name = ready[i];
        i += 1;
        if let Some(step) = step_by_name.get(name) {
            order.push(step);
        }
        if let Some(deps) = dependents.get(name) {
            let mut newly_ready: Vec<&str> = deps
                .iter()
                .filter(|&&dep| {
                    let d = in_degree.get_mut(dep).unwrap();
                    *d -= 1;
                    *d == 0
                })
                .copied()
                .collect();
            newly_ready.sort_by_key(|name| step_index.get(name).copied().unwrap_or(usize::MAX));
            ready.extend(newly_ready);
        }
    }

    if order.len() != steps.len() {
        return Err(WorkflowError::CyclicDependency(
            "one or more steps form a dependency cycle".to_string(),
        ));
    }

    Ok(order)
}

/// Execute a single step definition, running its referenced agent with the
/// step's goal as additional context.
///
/// # Errors
/// Returns `WorkflowError` if the agent is not found or execution fails.
pub async fn run_step(
    step: &crate::ast::StepDef,
    input: &str,
    ctx: &WorkflowContext<'_>,
) -> Result<StageResult, WorkflowError> {
    // Check approval gate before execution.
    // Use the injected handler (tests/CLI override) if present; otherwise
    // resolve a channel-appropriate handler from the approval definition.
    if let Some(approval_def) = &step.approval {
        let status = if let Some(handler) = &ctx.approval_handler {
            handler
                .request_approval(&step.name, input, approval_def)
                .await
        } else {
            let handler = crate::runtime::approval::resolve_approval_handler(approval_def);
            handler
                .request_approval(&step.name, input, approval_def)
                .await
        };
        match status {
            ApprovalStatus::Approved => {}
            ApprovalStatus::Rejected { reason } => {
                return Err(WorkflowError::ApprovalRejected {
                    step: step.name.clone(),
                    reason,
                });
            }
            ApprovalStatus::TimedOut | ApprovalStatus::Pending => {
                return Err(WorkflowError::ApprovalTimedOut {
                    step: step.name.clone(),
                });
            }
        }
    }

    let effective_input = if let Some(ref goal) = step.goal {
        format!("{input}\n\nGoal: {goal}")
    } else {
        input.to_string()
    };

    // run_stage now returns (StageResult, events); run_step discards the events
    // since its callers (run_step_with_fallback → run_steps) aggregate events separately.
    run_stage(&step.name, &step.agent, &effective_input, ctx)
        .await
        .map(|(result, _events)| result)
}

/// Execute a step, running its fallback if the primary step fails.
///
/// On primary failure, if `step.fallback` is set, the fallback step is executed
/// instead. Returns `(result, fallback_used)` — callers should emit a
/// `RunEvent::StepFallback` when `fallback_used` is `true`.
///
/// # Errors
/// Returns `WorkflowError` if both the primary and fallback steps fail,
/// or if the primary fails and no fallback is defined.
pub(crate) async fn run_step_with_fallback(
    step: &crate::ast::StepDef,
    input: &str,
    ctx: &WorkflowContext<'_>,
) -> Result<(StageResult, bool), WorkflowError> {
    match run_step(step, input, ctx).await {
        Ok(result) => Ok((result, false)),
        Err(e) => {
            if let Some(ref fallback) = step.fallback {
                let fallback_result = run_step(fallback, input, ctx).await?;
                Ok((fallback_result, true))
            } else {
                Err(e)
            }
        }
    }
}

/// Outcome of processing one step's result inside the `run_steps` loop.
enum StepOutcome {
    /// Step completed normally; continue to next step.
    Continue,
    /// `auto_resolve` conditions were met — break the loop early.
    AutoResolved,
    /// Hard error — propagate immediately and abort the workflow.
    HardError(WorkflowError),
}

/// Mutable loop state threaded through `run_steps` and `apply_step_result`.
///
/// Bundles the four collections that are mutated on every iteration so
/// `apply_step_result` takes a single `&mut StepLoopState` rather than four
/// separate `&mut` parameters.
struct StepLoopState {
    outputs: std::collections::HashMap<String, String>,
    failed_steps: std::collections::HashSet<String>,
    events: Vec<super::RunEvent>,
    results: Vec<StageResult>,
}

/// Process the result of a single step execution, updating shared loop state.
///
/// Extracts the success/failure logic from the `run_steps` loop so that the
/// outer function stays within the project line-limit.
fn apply_step_result(
    step: &crate::ast::StepDef,
    step_result: Result<(StageResult, Vec<super::RunEvent>), WorkflowError>,
    workflow: &WorkflowDef,
    state: &mut StepLoopState,
) -> StepOutcome {
    match step_result {
        Ok((result, step_events)) => {
            state.events.extend(step_events);

            // Check workflow-level auto_resolve conditions after each step.
            let resolved = if let Some(ref ar) = workflow.auto_resolve {
                auto_resolve_matches(&result.output, ar)
            } else {
                None
            };

            state
                .outputs
                .insert(step.name.clone(), result.output.clone());

            if let Some(ref condition) = resolved {
                state.events.push(super::RunEvent::AutoResolved {
                    step: step.name.clone(),
                    condition: condition.clone(),
                });
                state.results.push(result);
                return StepOutcome::AutoResolved;
            }

            state.results.push(result);
            state.events.push(super::RunEvent::StepCompleted {
                step: step.name.clone(),
            });
            StepOutcome::Continue
        }
        Err(e) => {
            // Hard errors abort immediately; soft errors record the failure and
            // let the workflow continue, skipping dependent steps.
            if e.is_hard_error() {
                return StepOutcome::HardError(e);
            }
            let reason = e.to_string();
            state.failed_steps.insert(step.name.clone());
            // Insert an empty output so the step appears in `outputs` for
            // downstream input-building. Note: `outputs.get(dep)` will return
            // `Some("")` for failed entries — not `None` — so `filter_map`
            // in the input-building block will include these as empty strings.
            // The skip-guard above ensures dependent steps never run with
            // this empty output.
            state.outputs.insert(step.name.clone(), String::new());
            state.events.push(super::RunEvent::StepFailed {
                step: step.name.clone(),
                reason,
            });
            state.results.push(StageResult {
                stage_name: step.name.clone(),
                agent_name: SENTINEL_FAILED.to_string(),
                output: String::new(),
                cost_cents: 0,
                tokens: 0,
            });
            StepOutcome::Continue
        }
    }
}

/// Execute all step blocks in a workflow, resolving `depends_on` ordering.
///
/// Steps are executed in topological order. Each step receives the concatenated
/// outputs of its declared dependencies as input context.
///
/// - Steps with `fallback` retry on failure using the fallback agent.
/// - Steps with `for_each` iterate over a JSON array in the trigger input.
///   Errors from `for_each` steps propagate to the `run_steps` loop via `?`
///   and are then classified by `apply_step_result`: soft errors (agent not
///   found, LLM failure) are absorbed — the step is recorded as failed and
///   dependent steps are skipped; hard errors abort the workflow.
/// - If `workflow.auto_resolve` conditions are met after a step, remaining
///   steps are short-circuited.
/// - Hard errors (`ApprovalRejected`, `ApprovalTimedOut`, `CyclicDependency`)
///   abort the workflow immediately. Soft errors record the failure and continue.
///
/// # Errors
/// Returns `WorkflowError` for hard errors or dependency cycles.
pub async fn run_steps(
    workflow: &WorkflowDef,
    ctx: &WorkflowContext<'_>,
) -> Result<(Vec<StageResult>, Vec<super::RunEvent>), WorkflowError> {
    use std::collections::{HashMap, HashSet};

    let ordered = resolve_dag(&workflow.steps)?;
    let trigger_input = format!("Trigger: {}", workflow.trigger);

    let mut state = StepLoopState {
        outputs: HashMap::new(),
        // Track steps that failed so their dependents can be skipped.
        failed_steps: HashSet::new(),
        results: Vec::new(),
        events: Vec::new(),
    };

    for (index, step) in ordered.into_iter().enumerate() {
        // Skip this step if any declared dependency failed.
        if let Some(failed_dep) = step
            .depends_on
            .iter()
            .find(|dep| state.failed_steps.contains(*dep))
        {
            let reason = format!("dependency '{failed_dep}' failed");
            state.events.push(super::RunEvent::StepSkipped {
                step: step.name.clone(),
                failed_dependency: failed_dep.clone(),
                reason,
            });
            // Insert an empty output so the step appears in `outputs` for
            // downstream input-building. `outputs.get(dep)` returns `Some("")`
            // for skipped entries, not `None`. Dependent steps are prevented
            // from running by the skip-guard (via failed_steps below).
            state.outputs.insert(step.name.clone(), String::new());
            state.results.push(StageResult {
                stage_name: step.name.clone(),
                agent_name: SENTINEL_SKIPPED.to_string(),
                output: String::new(),
                cost_cents: 0,
                tokens: 0,
            });
            // Also mark this step as failed so its own dependents are skipped.
            state.failed_steps.insert(step.name.clone());
            continue;
        }

        // Safety invariant: this block is only reached when every entry in
        // `step.depends_on` succeeded (the skip-guard above ensures any step
        // with a failed/skipped dependency is `continue`d before here).
        // Therefore `outputs.get(dep)` will not return `Some("")` for any dep.
        state.events.push(super::RunEvent::StepStarted {
            step: step.name.clone(),
            index,
        });

        let input = if step.depends_on.is_empty() {
            trigger_input.clone()
        } else {
            step.depends_on
                .iter()
                .filter_map(|dep| state.outputs.get(dep))
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        };

        // For for_each, prefer the raw trigger JSON (no "Trigger: " prefix) so
        // serde_json can parse it. Falls back to `input` when depends_on is set
        // (previous step output is already plain text/JSON).
        let for_each_input = if step.depends_on.is_empty() {
            workflow.trigger.clone()
        } else {
            input.clone()
        };

        let step_result = if let Some(ref key) = step.for_each {
            run_step_for_each(step, &for_each_input, key, ctx).await
        } else {
            run_step_with_fallback(step, &input, ctx)
                .await
                .map(|(r, fallback_used)| {
                    let mut evts = Vec::new();
                    if fallback_used {
                        let fallback_name = step
                            .fallback
                            .as_ref()
                            .expect(
                                "run_step: fallback_used is only true when step.fallback is Some",
                            )
                            .name
                            .clone();
                        evts.push(super::RunEvent::StepFallback {
                            step: step.name.clone(),
                            fallback_step: fallback_name,
                        });
                    }
                    (r, evts)
                })
        };

        match apply_step_result(step, step_result, workflow, &mut state) {
            StepOutcome::Continue => {}
            StepOutcome::AutoResolved => break,
            StepOutcome::HardError(e) => return Err(e),
        }
    }

    Ok((state.results, state.events))
}

/// Execute a step once per item in a JSON array extracted from the input.
///
/// Parses `input` as JSON, extracts the array at `collection_key`, then runs
/// the step for each element. Iteration outputs are aggregated as a JSON array
/// string (to avoid ambiguity with newlines in LLM output).
///
/// **All-or-nothing semantics**: a soft error on *any* iteration (agent not
/// found, provider failure) aborts the remaining iterations and propagates the
/// error to `run_steps`, which records the whole step as failed. Partial
/// results from completed iterations are discarded. Per-iteration partial
/// success is not supported; track that as a future improvement if needed.
///
/// If the JSON key is missing or the input is not valid JSON, the step is
/// executed once with the full input. This is intentional: callers with a
/// JSON trigger will get iteration; callers with plain text get a single pass.
///
/// **Approval per iteration**: if `step.approval` is set, the approval gate is
/// evaluated once per iteration (not once before the loop). Each item must
/// independently receive approval before its LLM call proceeds. If approval is
/// rejected for any iteration, that iteration returns a `WorkflowError` and the
/// loop is aborted — subsequent items are not processed.
///
/// Returns `(StageResult, Vec<RunEvent>)`. The caller is responsible for
/// inserting the returned events into the workflow trace.
async fn run_step_for_each(
    step: &crate::ast::StepDef,
    input: &str,
    collection_key: &str,
    ctx: &WorkflowContext<'_>,
) -> Result<(StageResult, Vec<super::RunEvent>), WorkflowError> {
    // Try to parse the trigger as JSON and extract the array.
    let items: Vec<String> = serde_json::from_str::<serde_json::Value>(input)
        .ok()
        .and_then(|v| v.get(collection_key).cloned())
        .and_then(|arr| arr.as_array().cloned())
        .map(|arr| {
            arr.iter()
                .map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_string))
                .collect()
        })
        .unwrap_or_default();

    // If the key wasn't found or empty, fall back to running the step once.
    if items.is_empty() {
        let (result, fallback_used) = run_step_with_fallback(step, input, ctx).await?;
        let mut evts = Vec::new();
        if fallback_used {
            let fallback_name = step
                .fallback
                .as_ref()
                .expect("run_step_for_each: fallback_used is only true when step.fallback is Some (empty items path)")
                .name
                .clone();
            evts.push(super::RunEvent::StepFallback {
                step: step.name.clone(),
                fallback_step: fallback_name,
            });
        }
        return Ok((result, evts));
    }

    let total = items.len();
    let mut outputs: Vec<serde_json::Value> = Vec::with_capacity(total);
    let mut total_cost = 0u64;
    let mut total_tokens = 0u64;
    let mut events: Vec<super::RunEvent> = Vec::new();

    for (index, item) in items.iter().enumerate() {
        let (result, fallback_used) = run_step_with_fallback(step, item, ctx).await?;
        total_cost += result.cost_cents;
        total_tokens += result.tokens;
        outputs.push(serde_json::Value::String(result.output));

        events.push(super::RunEvent::ForEachIteration {
            step: step.name.clone(),
            index,
            total,
        });

        if fallback_used {
            let fallback_name = step
                .fallback
                .as_ref()
                .expect("run_step_for_each: fallback_used is only true when step.fallback is Some (iteration path)")
                .name
                .clone();
            events.push(super::RunEvent::StepFallback {
                step: step.name.clone(),
                fallback_step: fallback_name,
            });
        }
    }

    // Aggregate outputs as a JSON array to avoid newline ambiguity.
    let aggregated = serde_json::to_string(&serde_json::Value::Array(outputs))
        .expect("Value::Array of Value::String is always serializable");

    Ok((
        StageResult {
            stage_name: step.name.clone(),
            agent_name: step.agent.clone(),
            output: aggregated,
            cost_cents: total_cost,
            tokens: total_tokens,
        },
        events,
    ))
}

/// Evaluate `auto_resolve` conditions against a step's output (parsed as JSON).
///
/// **AND semantics**: ALL conditions in the block must be satisfied. The first
/// failing condition short-circuits to `None`. There is no OR combinator at the
/// block level; add multiple `auto_resolve` blocks at the workflow level if OR
/// behavior is needed (not yet supported — tracked as a future issue).
///
/// Returns a human-readable description of the matched condition set, or `None`
/// if any condition is not met or the output cannot be parsed as JSON.
fn auto_resolve_matches(output: &str, ar: &crate::ast::AutoResolveBlock) -> Option<String> {
    use crate::ast::{AutoResolveCondition, CompareOp};

    // An empty conditions block must never trigger resolution.
    if ar.conditions.is_empty() {
        return None;
    }

    let json: serde_json::Value = serde_json::from_str(output).ok()?;

    for condition in &ar.conditions {
        match condition {
            AutoResolveCondition::Comparison(cmp) => {
                let field_val = json.get(&cmp.field)?.as_f64()?;
                let threshold: f64 = match &cmp.value {
                    crate::ast::WhenValue::Number(s) | crate::ast::WhenValue::Percent(s) => {
                        s.parse().ok()?
                    }
                    crate::ast::WhenValue::String(_) | crate::ast::WhenValue::Ident(_) => {
                        return None;
                    }
                    crate::ast::WhenValue::Currency { .. } => return None,
                };
                let matched = match cmp.op {
                    CompareOp::Gt => field_val > threshold,
                    CompareOp::Lt => field_val < threshold,
                    CompareOp::GtEq => field_val >= threshold,
                    CompareOp::LtEq => field_val <= threshold,
                    CompareOp::Eq => (field_val - threshold).abs() < f64::EPSILON,
                    CompareOp::NotEq => (field_val - threshold).abs() >= f64::EPSILON,
                };
                if !matched {
                    return None;
                }
            }
            AutoResolveCondition::IsOneOf { field, variants } => {
                let field_val = json.get(field)?.as_str()?;
                if !variants.iter().any(|v| v == field_val) {
                    return None;
                }
            }
        }
    }

    // All conditions passed — build a human-readable description for the trace event.
    let desc = ar
        .conditions
        .iter()
        .map(|c| match c {
            AutoResolveCondition::Comparison(cmp) => {
                let op_str = match cmp.op {
                    CompareOp::Gt => ">",
                    CompareOp::Lt => "<",
                    CompareOp::GtEq => ">=",
                    CompareOp::LtEq => "<=",
                    CompareOp::Eq => "==",
                    CompareOp::NotEq => "!=",
                };
                let val_str = match &cmp.value {
                    crate::ast::WhenValue::Number(s)
                    | crate::ast::WhenValue::Percent(s)
                    | crate::ast::WhenValue::String(s)
                    | crate::ast::WhenValue::Ident(s) => s.clone(),
                    crate::ast::WhenValue::Currency { symbol, amount } => {
                        format!("{symbol}{amount}")
                    }
                };
                format!("{} {op_str} {val_str}", cmp.field)
            }
            AutoResolveCondition::IsOneOf { field, variants } => {
                format!("{field} in [{}]", variants.join(", "))
            }
        })
        .collect::<Vec<_>>()
        .join(" AND ");

    Some(desc)
}

/// Execute a workflow using its declared execution mode.
/// If the workflow has step blocks, those are executed after stages.
///
/// # Errors
/// Returns `WorkflowError` if execution fails.
pub async fn run_workflow(
    workflow: &WorkflowDef,
    ctx: &WorkflowContext<'_>,
) -> Result<WorkflowResult, WorkflowError> {
    let mut result = match workflow.mode {
        ExecutionMode::Sequential => run_sequential(workflow, ctx).await?,
        ExecutionMode::Parallel => run_parallel(workflow, ctx).await?,
    };

    // Execute step blocks if present
    if !workflow.steps.is_empty() {
        let (step_results, step_events) = run_steps(workflow, ctx).await?;
        for sr in step_results {
            result.total_cost_cents += sr.cost_cents;
            result.total_tokens += sr.tokens;
            // Only propagate non-empty output so that failed/skipped steps
            // (which produce "" via their sentinel StageResult) do not
            // overwrite the last real output with an empty string.
            if !sr.output.is_empty() {
                result.final_output.clone_from(&sr.output);
            }
            result.stage_results.push(sr);
        }
        result.events.extend(step_events);
    }

    Ok(result)
}
