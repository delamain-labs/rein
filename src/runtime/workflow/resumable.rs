use std::collections::HashSet;

use crate::ast::WorkflowDef;

use super::persistence;
use super::{
    StageResult, WorkflowContext, WorkflowError, WorkflowResult, build_result, resolve_next_stage,
    run_stage,
};

fn build_resume_context(
    checkpoint: Option<&persistence::WorkflowState>,
    workflow: &WorkflowDef,
) -> (Vec<StageResult>, String, HashSet<String>) {
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
    (Vec::new(), default_input, HashSet::new())
}

/// Determine the stage to (re)start from given a checkpoint.
fn find_resume_start<'a>(
    workflow: &'a WorkflowDef,
    checkpoint: Option<&persistence::WorkflowState>,
    skip_stages: &HashSet<String>,
) -> Result<Option<&'a crate::ast::Stage>, WorkflowError> {
    if skip_stages.is_empty() {
        return Ok(workflow.stages.first());
    }
    let Some(last_cs) = checkpoint.and_then(|s| s.completed_stages.last()) else {
        return Ok(workflow.stages.first());
    };
    let Some(last_stage) = workflow.find_stage(&last_cs.stage_name) else {
        return Ok(workflow.stages.first());
    };
    resolve_next_stage(workflow, last_stage, &last_cs.output)
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
/// references a nonexistent stage, circular routing is detected, or state
/// persistence fails.
pub async fn run_sequential_resumable(
    workflow: &WorkflowDef,
    ctx: &WorkflowContext<'_>,
    state_path: &std::path::Path,
) -> Result<WorkflowResult, WorkflowError> {
    use persistence::{CompletedStage, WorkflowState, clear_state, load_state, save_state};

    let checkpoint =
        load_state(state_path).map_err(|e| WorkflowError::PersistenceFailure(e.to_string()))?;

    let (mut stage_results, mut current_input, skip_stages) =
        build_resume_context(checkpoint.as_ref(), workflow);
    let mut visited: HashSet<&str> = skip_stages.iter().map(String::as_str).collect();
    let mut current_stage = find_resume_start(workflow, checkpoint.as_ref(), &skip_stages)?;

    while let Some(stage) = current_stage {
        if !visited.insert(&stage.name) {
            return Err(WorkflowError::CircularRoute(stage.name.clone()));
        }

        let result = run_stage(&stage.name, &stage.agent, &current_input, ctx).await?;

        current_input.clone_from(&result.output);
        let output = result.output.clone();
        stage_results.push(result);

        let state = WorkflowState {
            version: persistence::WORKFLOW_STATE_VERSION,
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

        current_stage = resolve_next_stage(workflow, stage, &output)?;
    }

    clear_state(state_path).map_err(|e| WorkflowError::PersistenceFailure(e.to_string()))?;

    let final_output = stage_results
        .last()
        .map(|r| r.output.clone())
        .unwrap_or_default();

    Ok(build_result(stage_results, final_output))
}
