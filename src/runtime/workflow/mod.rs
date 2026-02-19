use crate::ast::{ExecutionMode, ReinFile, WorkflowDef};

use super::engine::{AgentEngine, RunConfig};
use super::executor::ToolExecutor;
use super::permissions::ToolRegistry;
use super::provider::{Provider, ToolDef};

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
}

impl std::fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AgentNotFound(name) => write!(f, "agent not found: {name}"),
            Self::StageFailed { stage, error } => {
                write!(f, "stage '{stage}' failed: {error:?}")
            }
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
    let engine = AgentEngine::new(provider, executor, &registry, tool_defs.to_vec(), config.clone());

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

/// Execute a workflow sequentially: each stage's output becomes the next stage's input.
///
/// # Errors
/// Returns `WorkflowError` if an agent is missing or a stage fails.
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

    for stage in &workflow.stages {
        let result = run_stage(
            &stage.name, &stage.agent, &current_input,
            file, provider, executor, tool_defs, config,
        ).await?;

        current_input.clone_from(&result.output);
        stage_results.push(result);
    }

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
            &stage.name, &stage.agent, &trigger_input,
            file, provider, executor, tool_defs, config,
        ).await?;
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
