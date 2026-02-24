use std::sync::Arc;

use crate::ast::{ExecutionMode, ReinFile, RouteRule, Stage, WorkflowDef};

/// The execution outcome of a single workflow step.
///
/// `status` replaces the old sentinel-string pattern (`agent_name == "<failed>"`)
/// with an explicit, compiler-checked enum field. Use `StageResult::is_real_execution()`
/// as the single, canonical predicate; inspect `status` directly only when you need
/// to distinguish `Failed` from `Skipped`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum StageResultStatus {
    /// The step ran to completion (agent produced a response).
    Executed,
    /// The step failed with a soft error (agent not found, LLM error, etc.).
    Failed,
    /// The step was cascade-skipped because a dependency failed or was skipped.
    Skipped,
}
use crate::runtime::approval::{ApprovalHandler, ApprovalStatus};

use super::engine::{AgentEngine, RunConfig};
use super::executor::ToolExecutor;
use super::permissions::ToolRegistry;
use super::provider::{Provider, ToolDef};

mod condition;
pub mod persistence;
mod resumable;

use condition::{condition_matches, when_expr_matches};
pub use resumable::run_sequential_resumable;

#[cfg(test)]
mod tests;

/// Governance and observability options for a single workflow run.
///
/// Extracted from `WorkflowContext` to reduce the number of fields callers
/// must populate when only infrastructure dependencies are needed. Callers
/// that don't need any governance features can pass `RunOptions::default()`
/// instead of spelling out three `None` values. (#440)
#[derive(Default)]
pub struct RunOptions {
    /// Optional pre-resolved approval handler. When `None`, each step
    /// resolves its own handler from the `ApprovalDef` channel at runtime.
    pub approval_handler: Option<Arc<dyn ApprovalHandler>>,
    /// When set, approval decisions are wrapped with `AuditingApprovalHandler`
    /// so every `ApprovalRequested` / `ApprovalResolved` event is persisted.
    pub audit_log: Option<Arc<crate::runtime::audit::AuditLog>>,
    /// Name of the workflow being executed. Passed to `AuditingApprovalHandler`
    /// so audit entries record the originating workflow for compliance queries.
    pub workflow_name: Option<String>,
}

/// Bundles the shared dependencies needed to execute workflow stages.
///
/// Eliminates repetitive parameter lists across `run_sequential`,
/// `run_parallel`, `run_sequential_resumable`, and `run_workflow`.
///
/// Governance and observability fields (`approval_handler`, `audit_log`,
/// `workflow_name`) are grouped in [`RunOptions`] so callers that don't
/// need them can pass `RunOptions::default()` instead of three `None`
/// values. (#440)
pub struct WorkflowContext<'a> {
    pub file: &'a ReinFile,
    pub provider: &'a dyn Provider,
    pub executor: &'a dyn ToolExecutor,
    pub tool_defs: &'a [ToolDef],
    pub config: &'a RunConfig,
    pub options: RunOptions,
}

/// The result of a completed workflow run.
#[derive(Debug)]
pub struct WorkflowResult {
    /// Results from each stage, in order of execution.
    pub stage_results: Vec<StageResult>,
    /// The final output text.
    ///
    /// **Stage-based workflows** (`run_sequential` / `run_parallel`): the
    /// output of the last stage. Empty if the workflow produced no output.
    ///
    /// **Step-based workflows** (`run_steps`): the output of the **last step
    /// that ran to completion** in topological order (`status == Executed`).
    /// If the terminal step fails or is skipped, this retains the output of
    /// the last earlier step that succeeded. Empty string if all steps failed.
    ///
    /// **Mixed workflows** (stages followed by steps, e.g. via `run_sequential`
    /// returning a result that is then augmented by `run_steps`): steps
    /// override the stage output for each step that executes successfully.
    /// If all steps fail or are skipped, `final_output` retains the last
    /// successful stage output — it is **not** reset to empty. Shell consumers
    /// and tests should check `stage_results` and `events` to distinguish
    /// "all steps failed, output is from stages" from "last step succeeded".
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
    /// Unix-epoch timestamps in milliseconds, parallel to `events`.
    /// Stage-based events carry real timestamps from the agent `RunTrace`.
    /// Step-based workflow events (e.g. `StepStarted`) use 0 as a sentinel
    /// until per-event timestamps are added (tracked separately).
    pub event_timestamps_ms: Vec<u64>,
}

/// The result of a single stage within a workflow.
#[derive(Debug)]
pub struct StageResult {
    pub stage_name: String,
    pub agent_name: String,
    pub output: String,
    pub cost_cents: u64,
    pub tokens: u64,
    /// Execution outcome. Prefer `is_real_execution()` over matching `status`
    /// directly unless you need to distinguish `Failed` from `Skipped`.
    pub status: StageResultStatus,
}

impl StageResult {
    /// Returns `true` if this result represents a step that actually executed
    /// (as opposed to a step that failed softly or was cascade-skipped).
    #[must_use]
    pub fn is_real_execution(&self) -> bool {
        self.status == StageResultStatus::Executed
    }
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
    /// A stage timed out. This is a hard error — the workflow is aborted
    /// immediately rather than treating the timeout as a soft step failure.
    StageTimedOut {
        stage: String,
        partial_trace: super::RunTrace,
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
    /// A step's approval gate returned `Pending` (deferred / async approval).
    ///
    /// `Pending` means the approval has been dispatched to an async channel
    /// (e.g. Slack Block Kit, webhook) but no synchronous answer was received.
    /// The workflow cannot continue without a decision, so it aborts
    /// immediately. Callers should surface the step name so the operator can
    /// resubmit once the external approval is resolved.
    ApprovalPending { step: String },
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
            Self::StageTimedOut { stage, .. } => {
                write!(f, "stage '{stage}' timed out")
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
            Self::ApprovalPending { step } => {
                write!(
                    f,
                    "approval pending for step '{step}' - workflow aborted; retry after external approval is resolved"
                )
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
            // Provider timeout is hard: a hung provider will likely hang the
            // next stage too — abort early rather than silently continuing.
            Self::ApprovalRejected { .. }
            | Self::ApprovalTimedOut { .. }
            | Self::ApprovalPending { .. }
            | Self::CyclicDependency(_)
            | Self::CircularRoute(_)
            | Self::PersistenceFailure(_)
            | Self::StageNotFound(_)
            | Self::StageTimedOut { .. } => true,
            // Soft — step is recorded as failed; dependents are skipped.
            Self::AgentNotFound(_) | Self::StageFailed { .. } => false,
        }
    }

    /// Returns a stable, `snake_case` string identifying the error variant.
    ///
    /// Used to populate the `error_kind` field on `RunEvent::StepFailed` so that
    /// OTEL dashboards and alerting rules can distinguish failure modes without
    /// parsing the human-readable `reason` string with regex.
    ///
    /// Uses an exhaustive `match` (no wildcard) so that adding a new variant
    /// produces a compile error until its kind string is made explicit.
    #[must_use]
    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::AgentNotFound(_) => "agent_not_found",
            Self::StageFailed { .. } => "stage_failed",
            Self::StageTimedOut { .. } => "stage_timed_out",
            Self::StageNotFound(_) => "stage_not_found",
            Self::PersistenceFailure(_) => "persistence_failure",
            Self::CircularRoute(_) => "circular_route",
            Self::ApprovalRejected { .. } => "approval_rejected",
            Self::ApprovalTimedOut { .. } => "approval_timed_out",
            Self::ApprovalPending { .. } => "approval_pending",
            Self::CyclicDependency(_) => "cyclic_dependency",
        }
    }

    /// Convert to the typed `StepErrorKind` enum for use in `RunEvent::StepFailed`.
    #[must_use]
    pub fn to_step_error_kind(&self) -> super::StepErrorKind {
        match self {
            Self::AgentNotFound(_) => super::StepErrorKind::AgentNotFound,
            Self::StageFailed { .. } => super::StepErrorKind::StageFailed,
            Self::StageTimedOut { .. } => super::StepErrorKind::StageTimedOut,
            Self::StageNotFound(_) => super::StepErrorKind::StageNotFound,
            Self::PersistenceFailure(_) => super::StepErrorKind::PersistenceFailure,
            Self::CircularRoute(_) => super::StepErrorKind::CircularRoute,
            Self::ApprovalRejected { .. } => super::StepErrorKind::ApprovalRejected,
            Self::ApprovalTimedOut { .. } => super::StepErrorKind::ApprovalTimedOut,
            Self::ApprovalPending { .. } => super::StepErrorKind::ApprovalPending,
            Self::CyclicDependency(_) => super::StepErrorKind::CyclicDependency,
        }
    }
}

impl std::error::Error for WorkflowError {}

/// Run a single stage and return its result plus raw timestamps.
pub(super) async fn run_stage(
    stage_name: &str,
    agent_name: &str,
    input: &str,
    ctx: &WorkflowContext<'_>,
) -> Result<(StageResult, Vec<super::RunEvent>, Vec<u64>), WorkflowError> {
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

    let result = engine.run(input).await.map_err(|e| {
        // #427: propagate timeout as a hard error so the workflow aborts
        // immediately rather than silently continuing to the next stage.
        if let super::RunError::Timeout { partial_trace } = e {
            WorkflowError::StageTimedOut {
                stage: stage_name.to_string(),
                partial_trace,
            }
        } else {
            WorkflowError::StageFailed {
                stage: stage_name.to_string(),
                error: e,
            }
        }
    })?;

    let timestamps = result.trace.timestamps_ms.clone();
    let events = result.trace.events;
    Ok((
        StageResult {
            stage_name: stage_name.to_string(),
            agent_name: agent_name.to_string(),
            output: result.response,
            cost_cents: result.total_cost_cents,
            tokens: result.total_tokens,
            status: StageResultStatus::Executed,
        },
        events,
        timestamps,
    ))
}

/// Collect stage results into a `WorkflowResult`.
pub(super) fn build_result(
    stage_results: Vec<StageResult>,
    final_output: String,
    events: Vec<super::RunEvent>,
    event_timestamps_ms: Vec<u64>,
) -> WorkflowResult {
    debug_assert_eq!(
        events.len(),
        event_timestamps_ms.len(),
        "event_timestamps_ms must be parallel to events (one entry per event)"
    );
    let total_cost = stage_results.iter().map(|r| r.cost_cents).sum();
    let total_tokens = stage_results.iter().map(|r| r.tokens).sum();
    WorkflowResult {
        stage_results,
        final_output,
        total_cost_cents: total_cost,
        total_tokens,
        events,
        event_timestamps_ms,
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
) -> Result<WorkflowResult, (WorkflowError, Vec<super::RunEvent>)> {
    let mut stage_results = Vec::new();
    let mut all_events: Vec<super::RunEvent> = Vec::new();
    let mut all_timestamps: Vec<u64> = Vec::new();
    let mut current_input = format!("Trigger: {}", workflow.trigger);
    let mut visited = std::collections::HashSet::new();

    let mut current_stage: Option<&Stage> = workflow.stages.first();

    while let Some(stage) = current_stage {
        if !visited.insert(&*stage.name) {
            return Err((WorkflowError::CircularRoute(stage.name.clone()), all_events));
        }

        let (result, stage_events, stage_timestamps) =
            match run_stage(&stage.name, &stage.agent, &current_input, ctx).await {
                Ok(r) => r,
                Err(WorkflowError::StageTimedOut { stage, partial_trace }) => {
                    // #420: surface partial trace events from the timed-out stage so
                    // callers can see what the stage did before the timeout (e.g.
                    // the `StageTimeout` event itself). Clone so the error also
                    // retains its own copy for display/log purposes.
                    all_events.extend(partial_trace.events.clone());
                    return Err((WorkflowError::StageTimedOut { stage, partial_trace }, all_events));
                }
                Err(e) => return Err((e, all_events.clone())),
            };

        current_input.clone_from(&result.output);
        let output = result.output.clone();
        stage_results.push(result);
        all_events.extend(stage_events);
        all_timestamps.extend(stage_timestamps);

        current_stage = resolve_next_stage(workflow, stage, &output)
            .map_err(|e| (e, all_events.clone()))?;
    }

    let final_output = stage_results
        .last()
        .map(|r| r.output.clone())
        .unwrap_or_default();

    Ok(build_result(
        stage_results,
        final_output,
        all_events,
        all_timestamps,
    ))
}

/// Execute all workflow stages with the trigger as input (fan-out pattern).
///
/// Stages run **concurrently** — each receives the same trigger input
/// independently and all are polled simultaneously. Results are returned in
/// stage-declaration order regardless of completion order. The first stage
/// error short-circuits and returns immediately.
///
/// # Errors
/// Returns `(WorkflowError, Vec<RunEvent>)` if an agent is missing or any
/// stage fails. Because `try_join_all` short-circuits on the first failure,
/// the partial-events vec is always empty for parallel aborts (no stages can
/// be guaranteed to have completed before the failing one is cancelled).
pub async fn run_parallel(
    workflow: &WorkflowDef,
    ctx: &WorkflowContext<'_>,
) -> Result<WorkflowResult, (WorkflowError, Vec<super::RunEvent>)> {
    use futures::future::join_all;

    let trigger_input = format!("Trigger: {}", workflow.trigger);

    // #457: use join_all (not try_join_all) so all stages run to completion
    // regardless of individual failures. Soft errors (AgentNotFound,
    // StageFailed) are recorded as Failed stages; hard errors abort.
    #[allow(clippy::type_complexity)]
    let raw: Vec<Result<(StageResult, Vec<super::RunEvent>, Vec<u64>), WorkflowError>> = join_all(
        workflow
            .stages
            .iter()
            .map(|stage| run_stage(&stage.name, &stage.agent, &trigger_input, ctx)),
    )
    .await;

    let mut stage_results = Vec::with_capacity(raw.len());
    let mut all_events: Vec<super::RunEvent> = Vec::new();
    let mut all_timestamps: Vec<u64> = Vec::new();
    let mut hard_error: Option<WorkflowError> = None;

    // Zip results with stage definitions to recover the agent name on failure.
    for (outcome, stage) in raw.into_iter().zip(workflow.stages.iter()) {
        match outcome {
            Ok((result, stage_events, stage_timestamps)) => {
                stage_results.push(result);
                all_events.extend(stage_events);
                all_timestamps.extend(stage_timestamps);
            }
            Err(e) if e.is_hard_error() => {
                // Surface the first hard error after collecting all results.
                if hard_error.is_none() {
                    hard_error = Some(e);
                }
            }
            Err(e) => {
                // Soft failure — record the stage as Failed and continue.
                let error_kind = e.to_step_error_kind();
                let reason = e.to_string();
                all_events.push(super::RunEvent::StepFailed {
                    step: stage.name.clone(),
                    reason,
                    error_kind,
                });
                stage_results.push(StageResult {
                    stage_name: stage.name.clone(),
                    agent_name: stage.agent.clone(),
                    output: String::new(),
                    cost_cents: 0,
                    tokens: 0,
                    status: StageResultStatus::Failed,
                });
                // Timestamps: StepFailed has no individual timestamp here; push
                // a zero so events and event_timestamps_ms stay in sync.
                all_timestamps.push(0);
            }
        }
    }

    if let Some(e) = hard_error {
        return Err((e, all_events));
    }

    let final_output = stage_results
        .iter()
        .filter(|r| r.status == StageResultStatus::Executed)
        .map(|r| format!("[{}]: {}", r.stage_name, r.output))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(build_result(
        stage_results,
        final_output,
        all_events,
        all_timestamps,
    ))
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
/// `ctx.options.workflow_name` is passed to `AuditingApprovalHandler::with_context` so
/// audit entries record the workflow they belong to for compliance queries.
///
/// # Errors
/// Returns `WorkflowError` if the agent is not found or execution fails.
pub async fn run_step(
    step: &crate::ast::StepDef,
    input: &str,
    ctx: &WorkflowContext<'_>,
) -> Result<StageResult, WorkflowError> {
    // Check approval gate before execution.
    //
    // #411: Pre-injected handlers (`ctx.options.approval_handler.is_some()`) are
    // used as-is — the caller is responsible for audit-wrapping them before
    // assembling `WorkflowContext`. Channel-resolved handlers (when
    // `ctx.options.approval_handler` is `None`) are wrapped here when an audit
    // log is present, because run_step is the only site that knows the step's
    // channel.
    if let Some(approval_def) = &step.approval {
        let status = if let Some(h) = ctx.options.approval_handler.as_ref() {
            // Pre-injected (already audit-wrapped if the caller required it).
            h.request_approval(&step.name, input, approval_def).await
        } else {
            // Channel-resolved: build handler from approval def, optionally
            // wrapping with auditing so compliance consumers see the entry.
            let base = Arc::from(crate::runtime::approval::resolve_approval_handler(
                approval_def,
            ));
            if let Some(log) = &ctx.options.audit_log {
                crate::runtime::approval::AuditingApprovalHandler::with_context(
                    base,
                    Arc::clone(log),
                    ctx.options.workflow_name.as_deref(),
                    Some(step.agent.as_str()),
                )
                .request_approval(&step.name, input, approval_def)
                .await
            } else {
                base.request_approval(&step.name, input, approval_def).await
            }
        };
        match status {
            ApprovalStatus::Approved => {}
            ApprovalStatus::Rejected { reason } => {
                return Err(WorkflowError::ApprovalRejected {
                    step: step.name.clone(),
                    reason,
                });
            }
            ApprovalStatus::TimedOut => {
                return Err(WorkflowError::ApprovalTimedOut {
                    step: step.name.clone(),
                });
            }
            ApprovalStatus::Pending => {
                return Err(WorkflowError::ApprovalPending {
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

    // run_stage now returns (StageResult, events, timestamps); run_step discards both
    // since its callers (run_step_with_fallback → run_steps) aggregate events separately.
    run_stage(&step.name, &step.agent, &effective_input, ctx)
        .await
        .map(|(result, _events, _timestamps)| result)
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
    /// Steps that are blocked from running — either they failed (soft error)
    /// or were skipped because a dependency was blocked. Dependents of any
    /// step in this set are skipped via the skip-guard in `run_steps`.
    /// Named `blocked_steps` rather than `failed_steps` to make clear that
    /// the set includes cascade-skipped steps, not just steps that errored.
    blocked_steps: std::collections::HashSet<String>,
    events: Vec<super::RunEvent>,
    /// Wall-clock timestamps (ms from workflow-step phase start) for each
    /// entry in `events`. Always kept in sync: `events.len() == event_timestamps_ms.len()`.
    event_timestamps_ms: Vec<u64>,
    results: Vec<StageResult>,
    /// Reference instant for computing per-event elapsed times.
    start: std::time::Instant,
}

impl StepLoopState {
    /// Push a step-phase event with the current wall-clock timestamp.
    ///
    /// Always updates both `events` and `event_timestamps_ms` atomically so
    /// the two vecs stay in sync. Use this for all step-level events
    /// (`StepStarted`, `StepCompleted`, `StepFailed`, `StepSkipped`, etc.).
    fn push_event(&mut self, event: super::RunEvent) {
        // Saturating cast: u128→u64. Clamp to u64::MAX before truncating so
        // the result is well-defined. A workflow lasting > ~584 million years
        // is an acceptable limitation for a CLI tool.
        let elapsed_ms = u64::try_from(
            self.start
                .elapsed()
                .as_millis()
                .min(u128::from(u64::MAX)),
        )
        .unwrap_or(u64::MAX);
        self.events.push(event);
        self.event_timestamps_ms.push(elapsed_ms);
    }
}

/// Process the result of a single step execution, updating shared loop state.
///
/// Extracts the success/failure logic from the `run_steps` loop so that the
/// outer function stays within the project line-limit.
///
/// Accepts only the `auto_resolve` slice of the workflow definition rather than
/// the full `WorkflowDef` — the function only uses this field (ISP — #469).
fn apply_step_result(
    step: &crate::ast::StepDef,
    step_result: Result<(StageResult, Vec<super::RunEvent>), WorkflowError>,
    auto_resolve: Option<&crate::ast::AutoResolveBlock>,
    state: &mut StepLoopState,
) -> StepOutcome {
    match step_result {
        Ok((result, step_events)) => {
            // Push each step event (e.g. StepFallback) with a real timestamp.
            for event in step_events {
                state.push_event(event);
            }

            // Check workflow-level auto_resolve conditions after each step.
            let resolved = if let Some(ar) = auto_resolve {
                auto_resolve_matches(&result.output, ar)
            } else {
                None
            };

            state
                .outputs
                .insert(step.name.clone(), result.output.clone());

            if let Some(ref condition) = resolved {
                state.push_event(super::RunEvent::AutoResolved {
                    step: step.name.clone(),
                    condition: condition.clone(),
                });
                state.results.push(result);
                return StepOutcome::AutoResolved;
            }

            state.results.push(result);
            state.push_event(super::RunEvent::StepCompleted {
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
            let error_kind = e.to_step_error_kind();
            let reason = e.to_string();
            state.blocked_steps.insert(step.name.clone());
            // Insert an empty output so the step appears in `outputs` for
            // downstream input-building. Note: `outputs.get(dep)` will return
            // `Some("")` for failed entries — not `None` — so `filter_map`
            // in the input-building block will include these as empty strings.
            // This is safe only because the skip-guard at the top of this loop
            // prevents any step that declares a blocked step as a dependency
            // from executing. Do not read `outputs` outside of `run_steps`
            // without accounting for this invariant.
            state.outputs.insert(step.name.clone(), String::new());
            state.push_event(super::RunEvent::StepFailed {
                step: step.name.clone(),
                reason,
                error_kind,
            });
            state.results.push(StageResult {
                stage_name: step.name.clone(),
                agent_name: step.agent.clone(),
                output: String::new(),
                cost_cents: 0,
                tokens: 0,
                status: StageResultStatus::Failed,
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
/// - Hard errors (`ApprovalRejected`, `ApprovalTimedOut`, `ApprovalPending`, `CyclicDependency`)
///   abort the workflow immediately. Soft errors record the failure and continue.
///
/// # Errors
/// Returns `WorkflowError` for hard errors or dependency cycles.
///
/// ## Return type on hard abort
///
/// On success: `Ok((results, events))`.
///
/// On hard error: `Err((error, partial_events))`. The `partial_events` vec
/// includes a `RunEvent::WorkflowAborted` entry that carries the
/// `error_kind` and `reason` from the abort, enabling callers (e.g. the
/// stderr display in `run_workflow_mode`) to surface the abort cause. It
/// also includes any step events collected before the abort (e.g. `StepStarted`).
///
/// Callers that propagate the error without inspecting partial events can use
/// `.map_err(|(e, _)| e)?` to recover the original `WorkflowError`.
pub(crate) async fn run_steps(
    workflow: &WorkflowDef,
    ctx: &WorkflowContext<'_>,
) -> Result<
    (Vec<StageResult>, Vec<super::RunEvent>, Vec<u64>),
    (WorkflowError, Vec<super::RunEvent>),
> {
    use std::collections::{HashMap, HashSet};

    let ordered = resolve_dag(&workflow.steps).map_err(|e| {
        let aborted = super::RunEvent::WorkflowAborted {
            error_kind: e.kind_str().to_string(),
            reason: e.to_string(),
        };
        (e, vec![aborted])
    })?;
    let trigger_input = format!("Trigger: {}", workflow.trigger);

    let start = std::time::Instant::now();
    let mut state = StepLoopState {
        outputs: HashMap::new(),
        blocked_steps: HashSet::new(),
        results: Vec::new(),
        events: Vec::new(),
        event_timestamps_ms: Vec::new(),
        start,
    };

    for (index, step) in ordered.into_iter().enumerate() {
        match handle_skip_guard(step, &mut state) {
            SkipGuardResult::Proceed => {}
            SkipGuardResult::Skipped => continue,
            SkipGuardResult::RunFallback => {
                // #456: run fallback directly (primary already blocked).
                match run_fallback_on_skip(step, &trigger_input, ctx, &mut state).await {
                    StepOutcome::Continue => {}
                    StepOutcome::AutoResolved => break,
                    StepOutcome::HardError(e) => {
                        state.push_event(super::RunEvent::WorkflowAborted {
                            error_kind: e.kind_str().to_string(),
                            reason: e.to_string(),
                        });
                        return Err((e, state.events));
                    }
                }
                continue;
            }
        }
        if handle_when_guard(step, &mut state) {
            continue;
        }

        match execute_step_in_dag(step, index, &trigger_input, workflow, ctx, &mut state).await {
            StepOutcome::Continue => {}
            StepOutcome::AutoResolved => break,
            StepOutcome::HardError(e) => {
                state.push_event(super::RunEvent::WorkflowAborted {
                    error_kind: e.kind_str().to_string(),
                    reason: e.to_string(),
                });
                return Err((e, state.events));
            }
        }
    }

    Ok((state.results, state.events, state.event_timestamps_ms))
}

/// Execute the fallback step for a step whose dependency-chain was blocked.
///
/// Called by `run_steps` when `handle_skip_guard` returns `RunFallback`.
/// The primary step is skipped entirely; only `step.fallback` is executed.
///
/// Returns `StepOutcome::Continue` on both success and soft failure (the
/// caller is responsible for checking `blocked_steps` to determine cascade
/// behaviour). Returns `StepOutcome::HardError` on hard failure.
async fn run_fallback_on_skip(
    step: &crate::ast::StepDef,
    trigger_input: &str,
    ctx: &WorkflowContext<'_>,
    state: &mut StepLoopState,
) -> StepOutcome {
    let fallback = step
        .fallback
        .as_ref()
        .expect("run_fallback_on_skip only called when step.fallback.is_some()");

    let input = if step.depends_on.is_empty() {
        trigger_input.to_string()
    } else {
        step.depends_on
            .iter()
            .filter_map(|dep| state.outputs.get(dep))
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    };

    match run_step(fallback, &input, ctx).await {
        Ok(result) => {
            // Fallback succeeded — record the step as Executed.
            // Do NOT insert into blocked_steps so dependents still run.
            state.push_event(super::RunEvent::StepFallback {
                step: step.name.clone(),
                fallback_step: fallback.name.clone(),
            });
            state
                .outputs
                .insert(step.name.clone(), result.output.clone());
            state.results.push(StageResult {
                stage_name: step.name.clone(),
                agent_name: fallback.agent.clone(),
                output: result.output,
                cost_cents: result.cost_cents,
                tokens: result.tokens,
                status: StageResultStatus::Executed,
            });
            state.push_event(super::RunEvent::StepCompleted {
                step: step.name.clone(),
            });
            StepOutcome::Continue
        }
        Err(e) if e.is_hard_error() => StepOutcome::HardError(e),
        Err(e) => {
            // Fallback also failed (soft) — mark step as Failed and
            // cascade-block its dependents.
            let error_kind = e.to_step_error_kind();
            let reason = e.to_string();
            state.blocked_steps.insert(step.name.clone());
            state.outputs.insert(step.name.clone(), String::new());
            state.push_event(super::RunEvent::StepFailed {
                step: step.name.clone(),
                reason,
                error_kind,
            });
            state.results.push(StageResult {
                stage_name: step.name.clone(),
                agent_name: step.agent.clone(),
                output: String::new(),
                cost_cents: 0,
                tokens: 0,
                status: StageResultStatus::Failed,
            });
            StepOutcome::Continue
        }
    }
}

/// Outcome of the skip-guard check. Controls what `run_steps` does next.
enum SkipGuardResult {
    /// No blocked dependency — proceed to normal execution.
    Proceed,
    /// Blocked dependency, no fallback — step has been skipped and recorded.
    Skipped,
    /// Blocked dependency, fallback defined — caller must run the fallback step.
    RunFallback,
}

/// Apply cascade-skip logic for a step whose dependency is blocked.
///
/// - `Proceed` when the step has no blocked dependency (should execute normally).
/// - `Skipped` when the step is blocked and has no fallback — the step is
///   recorded as `Skipped` and added to `blocked_steps`.
/// - `RunFallback` when the step is blocked but has a fallback — the caller
///   must attempt `step.fallback` before deciding whether to skip or fail.
fn handle_skip_guard(step: &crate::ast::StepDef, state: &mut StepLoopState) -> SkipGuardResult {
    let Some(failed_dep) = step
        .depends_on
        .iter()
        .find(|dep| state.blocked_steps.contains(*dep))
    else {
        return SkipGuardResult::Proceed;
    };

    // #456: if a fallback is defined, hand control back to the caller so
    // it can attempt the fallback before deciding to skip or fail.
    if step.fallback.is_some() {
        return SkipGuardResult::RunFallback;
    }

    let reason = format!("dependency '{failed_dep}' failed");
    state.push_event(super::RunEvent::StepSkipped {
        step: step.name.clone(),
        blocked_dependency: Some(failed_dep.clone()),
        reason,
    });
    // Insert an empty output so the step appears in `outputs` for
    // downstream input-building. `outputs.get(dep)` returns `Some("")`
    // for skipped entries, not `None`. Dependent steps are prevented
    // from running by the skip-guard (via failed_steps below).
    state.outputs.insert(step.name.clone(), String::new());
    // StepStarted is intentionally omitted for skipped steps — a skipped
    // step never began execution, so the event lifecycle is:
    //   normal:  StepStarted → StepCompleted
    //   failed:  StepStarted → StepFailed
    //   skipped: StepSkipped (no StepStarted)
    // Do not add StepStarted here to avoid a spurious event before StepSkipped.
    state.results.push(StageResult {
        stage_name: step.name.clone(),
        agent_name: step.agent.clone(),
        output: String::new(),
        cost_cents: 0,
        tokens: 0,
        status: StageResultStatus::Skipped,
    });
    // Also mark this step as blocked so its own dependents are skipped.
    state.blocked_steps.insert(step.name.clone());
    SkipGuardResult::Skipped
}

/// Check the `when:` guard on a step against the current step outputs.
///
/// Returns `true` when the step was skipped because the guard evaluated to
/// `false` (caller should `continue` the loop), or `false` when the guard is
/// absent or evaluates to `true` and the step should proceed.
///
/// Unlike `handle_skip_guard`, a when:-skipped step does NOT cascade-block its
/// dependents — dependent steps should still execute with empty prior output.
fn handle_when_guard(step: &crate::ast::StepDef, state: &mut StepLoopState) -> bool {
    let Some(ref expr) = step.when else {
        return false;
    };

    if when_expr_matches(expr, &state.outputs) {
        // Condition satisfied — step should run.
        return false;
    }

    // Condition false → skip without cascade-blocking dependents.
    state.push_event(super::RunEvent::StepSkipped {
        step: step.name.clone(),
        blocked_dependency: None,
        reason: "when: condition false".to_string(),
    });
    state.outputs.insert(step.name.clone(), String::new());
    state.results.push(StageResult {
        stage_name: step.name.clone(),
        agent_name: step.agent.clone(),
        output: String::new(),
        cost_cents: 0,
        tokens: 0,
        status: StageResultStatus::Skipped,
    });
    // Do NOT insert into blocked_steps — dependents should still run.
    true
}

/// Execute one step in the DAG loop body.
///
/// Emits `StepStarted`, builds the step input from prior outputs or trigger,
/// runs the step (with `for_each` or fallback as applicable), then delegates to
/// `apply_step_result`. Callers must have already confirmed this step has no
/// blocked dependency (i.e. `handle_skip_guard` returned `None`).
async fn execute_step_in_dag(
    step: &crate::ast::StepDef,
    index: usize,
    trigger_input: &str,
    workflow: &WorkflowDef,
    ctx: &WorkflowContext<'_>,
    state: &mut StepLoopState,
) -> StepOutcome {
    // Safety invariant: this function is only called when every entry in
    // `step.depends_on` succeeded (the skip-guard ensures any step with a
    // failed/skipped dependency is handled before this call).
    // Therefore `outputs.get(dep)` will not return `Some("")` for any dep.
    state.push_event(super::RunEvent::StepStarted {
        step: step.name.clone(),
        index,
    });

    let input = if step.depends_on.is_empty() {
        trigger_input.to_string()
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

    apply_step_result(step, step_result, workflow.auto_resolve.as_ref(), state)
}

/// Execute a step once per item in a JSON array extracted from the input.
///
/// Parses `input` as JSON, extracts the array at `collection_key`, then runs
/// the step for each element. Iteration outputs are aggregated as a JSON array
/// string (to avoid ambiguity with newlines in LLM output).
///
/// **All-or-nothing semantics**: a soft error on any iteration aborts the
/// remaining iterations and propagates the error to `run_steps`, which records
/// the whole step as failed. Partial results from completed iterations are
/// discarded. Per-iteration partial success is not supported (tracked as #428).
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
        // #408: soft errors (AgentNotFound, StageFailed) skip the failing
        // iteration and continue with the remaining ones rather than aborting.
        // Hard errors (ApprovalRejected, StageTimedOut, etc.) still propagate.
        let (result, fallback_used) = match run_step_with_fallback(step, item, ctx).await {
            Ok(r) => r,
            Err(e) if !e.is_hard_error() => {
                events.push(super::RunEvent::ForEachIterationFailed {
                    step: step.name.clone(),
                    index,
                    total,
                    reason: e.to_string(),
                });
                continue;
            }
            Err(e) => return Err(e),
        };
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
            status: StageResultStatus::Executed,
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
/// Returns `(WorkflowError, Vec<RunEvent>)` on failure. The event vec carries
/// any events (including `WorkflowAborted`) that were emitted before the abort
/// so OTEL consumers can observe the hard-error cause.
pub async fn run_workflow(
    workflow: &WorkflowDef,
    ctx: &WorkflowContext<'_>,
) -> Result<WorkflowResult, (WorkflowError, Vec<super::RunEvent>)> {
    let mut result = match workflow.mode {
        ExecutionMode::Sequential => run_sequential(workflow, ctx).await.map_err(|(e, partial)| {
            let aborted = super::RunEvent::WorkflowAborted {
                error_kind: e.kind_str().to_string(),
                reason: e.to_string(),
            };
            let mut events = partial;
            events.push(aborted);
            (e, events)
        })?,
        ExecutionMode::Parallel => run_parallel(workflow, ctx).await.map_err(|(e, partial)| {
            let aborted = super::RunEvent::WorkflowAborted {
                error_kind: e.kind_str().to_string(),
                reason: e.to_string(),
            };
            let mut events = partial;
            events.push(aborted);
            (e, events)
        })?,
    };

    // Execute step blocks if present
    if !workflow.steps.is_empty() {
        // On hard abort, merge stage events already in result.events with the
        // step-phase partial events (including WorkflowAborted) so callers
        // receive full context of both what ran and why it aborted.
        let (step_results, step_events, step_timestamps) = match run_steps(workflow, ctx).await {
            Ok(ok) => ok,
            Err((e, step_partial)) => {
                let mut merged = std::mem::take(&mut result.events);
                merged.extend(step_partial);
                return Err((e, merged));
            }
        };
        for sr in step_results {
            result.total_cost_cents += sr.cost_cents;
            result.total_tokens += sr.tokens;
            // "Last real-execution wins" contract: final_output is updated for
            // each step whose status == Executed (in topological order). The
            // last Executed step in the order sets the output seen by callers.
            // If the terminal step in declaration order fails/is skipped, the
            // output of the last *successful* step is returned — this is
            // intentional and documented on `WorkflowResult::final_output`.
            // Steps with `status == Failed` or `status == Skipped` have an
            // empty output and must not overwrite a previous real result.
            if sr.is_real_execution() {
                result.final_output.clone_from(&sr.output);
            }
            result.stage_results.push(sr);
        }
        result.events.extend(step_events);
        result.event_timestamps_ms.extend(step_timestamps);
    }

    Ok(result)
}
