use std::sync::Arc;

use crate::ast::{ExecutionMode, ReinFile, RouteRule, Stage, WorkflowDef};
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
}

impl std::fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AgentNotFound(name) => write!(f, "agent not found: {name}"),
            Self::StageFailed { stage, error } => {
                write!(f, "stage '{stage}' failed: {error:?}")
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
) -> Result<StageResult, WorkflowError> {
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

    Ok(StageResult {
        stage_name: stage_name.to_string(),
        agent_name: agent_name.to_string(),
        output: result.response,
        cost_cents: result.total_cost_cents,
        tokens: result.total_tokens,
    })
}

/// Collect stage results into a `WorkflowResult`.
pub(super) fn build_result(
    stage_results: Vec<StageResult>,
    final_output: String,
) -> WorkflowResult {
    let total_cost = stage_results.iter().map(|r| r.cost_cents).sum();
    let total_tokens = stage_results.iter().map(|r| r.tokens).sum();
    WorkflowResult {
        stage_results,
        final_output,
        total_cost_cents: total_cost,
        total_tokens,
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
    let mut current_input = format!("Trigger: {}", workflow.trigger);
    let mut visited = std::collections::HashSet::new();

    let mut current_stage: Option<&Stage> = workflow.stages.first();

    while let Some(stage) = current_stage {
        if !visited.insert(&*stage.name) {
            return Err(WorkflowError::CircularRoute(stage.name.clone()));
        }

        let result = run_stage(&stage.name, &stage.agent, &current_input, ctx).await?;

        current_input.clone_from(&result.output);
        let output = result.output.clone();
        stage_results.push(result);

        current_stage = resolve_next_stage(workflow, stage, &output)?;
    }

    let final_output = stage_results
        .last()
        .map(|r| r.output.clone())
        .unwrap_or_default();

    Ok(build_result(stage_results, final_output))
}

/// Build initial execution state from a prior checkpoint, or return a fresh
/// state if none exists (or the checkpoint is for a different workflow).
///
/// Returns `(stage_results, current_input, skip_stages)`.
/// Execute all workflow stages with the trigger as input (fan-out pattern).
/// Each stage receives the same trigger input independently.
///
/// # Errors
/// Returns `WorkflowError` if an agent is missing or any stage fails.
pub async fn run_parallel(
    workflow: &WorkflowDef,
    ctx: &WorkflowContext<'_>,
) -> Result<WorkflowResult, WorkflowError> {
    let trigger_input = format!("Trigger: {}", workflow.trigger);
    let mut stage_results = Vec::new();

    for stage in &workflow.stages {
        let result = run_stage(&stage.name, &stage.agent, &trigger_input, ctx).await?;
        stage_results.push(result);
    }

    let final_output = stage_results
        .iter()
        .map(|r| format!("[{}]: {}", r.stage_name, r.output))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(build_result(stage_results, final_output))
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
    // Check approval gate before execution
    if let Some(approval_def) = &step.approval {
        if let Some(handler) = &ctx.approval_handler {
            let status = handler
                .request_approval(&step.name, input, approval_def)
                .await;
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
    }

    let effective_input = if let Some(ref goal) = step.goal {
        format!("{input}\n\nGoal: {goal}")
    } else {
        input.to_string()
    };

    run_stage(&step.name, &step.agent, &effective_input, ctx).await
}

/// Execute all step blocks in a workflow sequentially, chaining outputs.
///
/// # Errors
/// Returns `WorkflowError` if any step fails.
pub async fn run_steps(
    workflow: &WorkflowDef,
    ctx: &WorkflowContext<'_>,
) -> Result<Vec<StageResult>, WorkflowError> {
    let mut results = Vec::new();
    let mut current_input = format!("Trigger: {}", workflow.trigger);

    for step in &workflow.steps {
        let result = run_step(step, &current_input, ctx).await?;
        current_input.clone_from(&result.output);
        results.push(result);
    }

    Ok(results)
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
        let step_results = run_steps(workflow, ctx).await?;
        for sr in step_results {
            result.total_cost_cents += sr.cost_cents;
            result.total_tokens += sr.tokens;
            result.final_output.clone_from(&sr.output);
            result.stage_results.push(sr);
        }
    }

    Ok(result)
}
