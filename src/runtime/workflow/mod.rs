use crate::ast::{ReinFile, WorkflowDef};

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
    /// The final output text (from the last stage).
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
    let mut total_cost: u64 = 0;
    let mut total_tokens: u64 = 0;

    for stage in &workflow.stages {
        let agent = file
            .agents
            .iter()
            .find(|a| a.name == stage.agent)
            .ok_or_else(|| WorkflowError::AgentNotFound(stage.agent.clone()))?;

        let registry = ToolRegistry::from_agent(agent);

        let engine = AgentEngine::new(provider, executor, &registry, tool_defs.to_vec(), config.clone());

        let result = engine
            .run(&current_input)
            .await
            .map_err(|e| WorkflowError::StageFailed {
                stage: stage.name.clone(),
                error: e,
            })?;

        current_input.clone_from(&result.response);
        total_cost += result.total_cost_cents;
        total_tokens += result.total_tokens;

        stage_results.push(StageResult {
            stage_name: stage.name.clone(),
            agent_name: stage.agent.clone(),
            output: result.response,
            cost_cents: result.total_cost_cents,
            tokens: result.total_tokens,
        });
    }

    let final_output = stage_results
        .last()
        .map(|r| r.output.clone())
        .unwrap_or_default();

    Ok(WorkflowResult {
        stage_results,
        final_output,
        total_cost_cents: total_cost,
        total_tokens,
    })
}
