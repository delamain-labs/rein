use crate::ast::{ExecutionMode, ReinFile, RouteRule, Stage, WorkflowDef};

use super::engine::{AgentEngine, RunConfig};
use super::executor::ToolExecutor;
use super::permissions::ToolRegistry;
use super::provider::{Provider, ToolDef};

pub mod persistence;

#[cfg(test)]
mod tests;

/// The result of a completed workflow run.
#[derive(Debug)]
pub struct WorkflowResult {
    /// Results from each stage, in order.
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
        }
    }
}

impl std::error::Error for WorkflowError {}

/// Run a single stage and return its result.
#[allow(clippy::too_many_arguments)]
async fn run_stage(
    stage_name: &str,
    agent_name: &str,
    input: &str,
    file: &ReinFile,
    provider: &dyn Provider,
    executor: &dyn ToolExecutor,
    tool_defs: &[ToolDef],
    config: &RunConfig,
) -> Result<StageResult, WorkflowError> {
    let agent = file
        .agents
        .iter()
        .find(|a| a.name == agent_name)
        .ok_or_else(|| WorkflowError::AgentNotFound(agent_name.to_string()))?;

    let registry = ToolRegistry::from_agent(agent);
    let engine = AgentEngine::new(
        provider,
        executor,
        &registry,
        tool_defs.to_vec(),
        config.clone(),
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
fn build_result(stage_results: Vec<StageResult>, final_output: String) -> WorkflowResult {
    let total_cost = stage_results.iter().map(|r| r.cost_cents).sum();
    let total_tokens = stage_results.iter().map(|r| r.tokens).sum();
    WorkflowResult {
        stage_results,
        final_output,
        total_cost_cents: total_cost,
        total_tokens,
    }
}

/// Check whether a conditional route matches the agent output.
///
/// Looks for `field: value` or `field=value` patterns in the output text.
fn condition_matches(output: &str, field: &str, equals: &str) -> bool {
    let lower = output.to_lowercase();
    let field_lower = field.to_lowercase();
    let equals_lower = equals.to_lowercase();

    for line in lower.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&field_lower) {
            let rest = rest.trim_start();
            if rest
                .strip_prefix(':')
                .is_some_and(|val| val.trim().starts_with(&equals_lower))
            {
                return true;
            }
            if rest
                .strip_prefix('=')
                .is_some_and(|val| val.trim().starts_with(&equals_lower))
            {
                return true;
            }
        }
    }
    false
}

/// Find a stage by name within a workflow.
fn find_stage<'a>(workflow: &'a WorkflowDef, name: &str) -> Option<&'a Stage> {
    workflow.stages.iter().find(|s| s.name == name)
}

/// Execute a workflow sequentially: each stage's output becomes the next
/// stage's input. Respects [`RouteRule::Conditional`] for branching to named
/// stages.
///
/// # Errors
/// Returns `WorkflowError` if an agent is missing, a stage fails, or a route
/// references a nonexistent stage.
pub async fn run_sequential(
    workflow: &WorkflowDef,
    file: &ReinFile,
    provider: &dyn Provider,
    executor: &dyn ToolExecutor,
    tool_defs: &[ToolDef],
    config: &RunConfig,
) -> Result<WorkflowResult, WorkflowError> {
    let mut stage_results = Vec::new();
    let mut current_input = format!("Trigger: {}", workflow.trigger);
    let mut visited = std::collections::HashSet::new();

    let mut current_stage: Option<&Stage> = workflow.stages.first();

    while let Some(stage) = current_stage {
        if !visited.insert(stage.name.clone()) {
            break; // circular routing protection
        }

        let result = run_stage(
            &stage.name,
            &stage.agent,
            &current_input,
            file,
            provider,
            executor,
            tool_defs,
            config,
        )
        .await?;

        current_input.clone_from(&result.output);
        let output = result.output.clone();
        stage_results.push(result);

        current_stage = match &stage.route {
            RouteRule::Next => {
                let idx = workflow.stages.iter().position(|s| s.name == stage.name);
                idx.and_then(|i| workflow.stages.get(i + 1))
            }
            RouteRule::Conditional {
                field,
                equals,
                then_stage,
                else_stage,
            } => {
                if condition_matches(&output, field, equals) {
                    Some(
                        find_stage(workflow, then_stage)
                            .ok_or_else(|| WorkflowError::StageNotFound(then_stage.clone()))?,
                    )
                } else if let Some(else_name) = else_stage {
                    Some(
                        find_stage(workflow, else_name)
                            .ok_or_else(|| WorkflowError::StageNotFound(else_name.clone()))?,
                    )
                } else {
                    None
                }
            }
        };
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
fn build_resume_context(
    checkpoint: Option<&persistence::WorkflowState>,
    workflow: &WorkflowDef,
) -> (Vec<StageResult>, String, std::collections::HashSet<String>) {
    let default_input = format!("Trigger: {}", workflow.trigger);
    if let Some(state) = checkpoint.filter(|s| s.workflow_name == workflow.name) {
        let prior_results = state
            .completed_stages
            .iter()
            .map(|cs| StageResult {
                stage_name: cs.stage_name.clone(),
                agent_name: cs.agent_name.clone(),
                output: cs.output.clone(),
                cost_cents: cs.cost_cents,
                tokens: cs.tokens,
            })
            .collect();
        let skip = state
            .completed_stages
            .iter()
            .map(|cs| cs.stage_name.clone())
            .collect();
        return (prior_results, state.next_input.clone(), skip);
    }
    (Vec::new(), default_input, std::collections::HashSet::new())
}

/// Determine the stage to (re)start from given a checkpoint.
///
/// If no stages have been skipped, execution starts from the first stage.
/// Otherwise the last completed stage's route decides the next one.
fn find_resume_start<'a>(
    workflow: &'a WorkflowDef,
    checkpoint: Option<&persistence::WorkflowState>,
    skip_stages: &std::collections::HashSet<String>,
) -> Option<&'a Stage> {
    if skip_stages.is_empty() {
        return workflow.stages.first();
    }
    let Some(last_cs) = checkpoint.and_then(|s| s.completed_stages.last()) else {
        return workflow.stages.first();
    };
    let last_stage = find_stage(workflow, &last_cs.stage_name)?;
    match &last_stage.route {
        RouteRule::Next => {
            let idx = workflow
                .stages
                .iter()
                .position(|s| s.name == last_stage.name)?;
            workflow.stages.get(idx + 1)
        }
        RouteRule::Conditional {
            field,
            equals,
            then_stage,
            else_stage,
        } => {
            let output = last_cs.output.as_str();
            if condition_matches(output, field, equals) {
                find_stage(workflow, then_stage)
            } else {
                else_stage
                    .as_ref()
                    .and_then(|name| find_stage(workflow, name))
            }
        }
    }
}

/// Execute a workflow sequentially with checkpoint persistence.
///
/// After each stage completes, state is saved to `state_path` as JSON.
/// If a previous state file exists for this workflow, execution resumes
/// from the last completed stage. On successful completion the state file
/// is removed.
///
/// # Errors
/// Returns `WorkflowError` if an agent is missing, a stage fails, a route
/// references a nonexistent stage, or state persistence fails.
#[allow(clippy::too_many_arguments)]
pub async fn run_sequential_resumable(
    workflow: &WorkflowDef,
    file: &ReinFile,
    provider: &dyn Provider,
    executor: &dyn ToolExecutor,
    tool_defs: &[ToolDef],
    config: &RunConfig,
    state_path: &std::path::Path,
) -> Result<WorkflowResult, WorkflowError> {
    use persistence::{CompletedStage, WorkflowState, clear_state, load_state, save_state};

    let checkpoint =
        load_state(state_path).map_err(|e| WorkflowError::PersistenceFailure(e.to_string()))?;

    let (mut stage_results, mut current_input, skip_stages) =
        build_resume_context(checkpoint.as_ref(), workflow);
    let mut visited: std::collections::HashSet<String> = skip_stages.iter().cloned().collect();
    let mut current_stage = find_resume_start(workflow, checkpoint.as_ref(), &skip_stages);

    while let Some(stage) = current_stage {
        if !visited.insert(stage.name.clone()) {
            break;
        }

        let result = run_stage(
            &stage.name,
            &stage.agent,
            &current_input,
            file,
            provider,
            executor,
            tool_defs,
            config,
        )
        .await?;

        current_input.clone_from(&result.output);
        let output = result.output.clone();
        stage_results.push(result);

        let state = WorkflowState {
            workflow_name: workflow.name.clone(),
            completed_stages: stage_results
                .iter()
                .map(|r| CompletedStage {
                    stage_name: r.stage_name.clone(),
                    agent_name: r.agent_name.clone(),
                    output: r.output.clone(),
                    cost_cents: r.cost_cents,
                    tokens: r.tokens,
                })
                .collect(),
            next_input: current_input.clone(),
        };
        save_state(&state, state_path)
            .map_err(|e| WorkflowError::PersistenceFailure(e.to_string()))?;

        current_stage = match &stage.route {
            RouteRule::Next => {
                let idx = workflow.stages.iter().position(|s| s.name == stage.name);
                idx.and_then(|i| workflow.stages.get(i + 1))
            }
            RouteRule::Conditional {
                field,
                equals,
                then_stage,
                else_stage,
            } => {
                if condition_matches(&output, field, equals) {
                    Some(
                        find_stage(workflow, then_stage)
                            .ok_or_else(|| WorkflowError::StageNotFound(then_stage.clone()))?,
                    )
                } else if let Some(else_name) = else_stage {
                    Some(
                        find_stage(workflow, else_name)
                            .ok_or_else(|| WorkflowError::StageNotFound(else_name.clone()))?,
                    )
                } else {
                    None
                }
            }
        };
    }

    clear_state(state_path).map_err(|e| WorkflowError::PersistenceFailure(e.to_string()))?;

    let final_output = stage_results
        .last()
        .map(|r| r.output.clone())
        .unwrap_or_default();

    Ok(build_result(stage_results, final_output))
}

/// Execute all workflow stages with the trigger as input (fan-out pattern).
/// Each stage receives the same trigger input independently.
///
/// # Errors
/// Returns `WorkflowError` if an agent is missing or any stage fails.
pub async fn run_parallel(
    workflow: &WorkflowDef,
    file: &ReinFile,
    provider: &dyn Provider,
    executor: &dyn ToolExecutor,
    tool_defs: &[ToolDef],
    config: &RunConfig,
) -> Result<WorkflowResult, WorkflowError> {
    let trigger_input = format!("Trigger: {}", workflow.trigger);
    let mut stage_results = Vec::new();

    // Fan-out: each stage gets the trigger input (not chained)
    for stage in &workflow.stages {
        let result = run_stage(
            &stage.name,
            &stage.agent,
            &trigger_input,
            file,
            provider,
            executor,
            tool_defs,
            config,
        )
        .await?;
        stage_results.push(result);
    }

    // Merge outputs
    let final_output = stage_results
        .iter()
        .map(|r| format!("[{}]: {}", r.stage_name, r.output))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(build_result(stage_results, final_output))
}

/// Execute a workflow using its declared execution mode.
///
/// # Errors
/// Returns `WorkflowError` if execution fails.
pub async fn run_workflow(
    workflow: &WorkflowDef,
    file: &ReinFile,
    provider: &dyn Provider,
    executor: &dyn ToolExecutor,
    tool_defs: &[ToolDef],
    config: &RunConfig,
) -> Result<WorkflowResult, WorkflowError> {
    match workflow.mode {
        ExecutionMode::Sequential => {
            run_sequential(workflow, file, provider, executor, tool_defs, config).await
        }
        ExecutionMode::Parallel => {
            run_parallel(workflow, file, provider, executor, tool_defs, config).await
        }
    }
}
