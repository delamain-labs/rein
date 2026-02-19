use crate::ast::{AgentDef, Constraint, ProviderDef, ReinFile, Span};

/// Severity of a diagnostic.
#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

/// A validation diagnostic (error or warning) with source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    fn error(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Self {
            severity: Severity::Error,
            code,
            message: message.into(),
            span,
        }
    }

    fn warning(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Self {
            severity: Severity::Warning,
            code,
            message: message.into(),
            span,
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

/// Run all validation passes on a parsed file.
/// Returns a list of diagnostics (errors and warnings).
pub fn validate(file: &ReinFile) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    check_duplicate_provider_names(file, &mut diags);
    for provider in &file.providers {
        check_provider_key_present(provider, &mut diags);
    }
    check_duplicate_agent_names(file, &mut diags);
    for agent in &file.agents {
        check_can_cannot_overlap(agent, &mut diags);
        check_budget_positive(agent, &mut diags);
        check_constraint_amounts(agent, &mut diags);
        check_duplicate_capabilities(agent, &mut diags);
        check_model_present(agent, &mut diags);
    }
    check_duplicate_workflow_names(file, &mut diags);
    for workflow in &file.workflows {
        check_workflow_stages_reference_agents(file, workflow, &mut diags);
        check_duplicate_stages(workflow, &mut diags);
        check_workflow_steps_reference_agents(file, workflow, &mut diags);
        check_duplicate_step_names(workflow, &mut diags);
        check_step_stage_name_collisions(workflow, &mut diags);
    }
    diags
}

/// E001: two agents with the same name.
fn check_duplicate_agent_names(file: &ReinFile, diags: &mut Vec<Diagnostic>) {
    use std::collections::HashMap;
    let mut seen: HashMap<&str, &AgentDef> = HashMap::new();
    for agent in &file.agents {
        if let Some(first) = seen.get(agent.name.as_str()) {
            diags.push(Diagnostic::error(
                "E001",
                format!(
                    "duplicate agent name '{}': first defined at {}",
                    agent.name, first.span.start
                ),
                agent.span.clone(),
            ));
        } else {
            seen.insert(agent.name.as_str(), agent);
        }
    }
}

/// E002: same tool appears in both `can` and `cannot`.
fn check_can_cannot_overlap(agent: &AgentDef, diags: &mut Vec<Diagnostic>) {
    use std::collections::HashSet;
    let allowed: HashSet<(&str, &str)> = agent
        .can
        .iter()
        .map(|c| (c.namespace.as_str(), c.action.as_str()))
        .collect();
    for denied in &agent.cannot {
        if allowed.contains(&(denied.namespace.as_str(), denied.action.as_str())) {
            diags.push(Diagnostic::error(
                "E002",
                format!(
                    "capability '{}.{}' appears in both `can` and `cannot` in agent '{}'",
                    denied.namespace, denied.action, agent.name
                ),
                denied.span.clone(),
            ));
        }
    }
}

/// E003: budget amount must be positive (non-zero cents).
fn check_budget_positive(agent: &AgentDef, diags: &mut Vec<Diagnostic>) {
    if let Some(budget) = &agent.budget
        && budget.amount == 0
    {
        diags.push(Diagnostic::error(
            "E003",
            format!(
                "budget amount must be positive, got 0 in agent '{}'",
                agent.name
            ),
            budget.span.clone(),
        ));
    }
}

/// E004: monetary constraint amount must be positive.
fn check_constraint_amounts(agent: &AgentDef, diags: &mut Vec<Diagnostic>) {
    for cap in agent.can.iter().chain(agent.cannot.iter()) {
        if let Some(Constraint::MonetaryCap { amount, .. }) = &cap.constraint
            && *amount == 0
        {
            diags.push(Diagnostic::error(
                "E004",
                format!(
                    "constraint amount must be positive in agent '{}'",
                    agent.name
                ),
                cap.span.clone(),
            ));
        }
    }
}

/// W003: duplicate capabilities within the same list.
fn check_duplicate_capabilities(agent: &AgentDef, diags: &mut Vec<Diagnostic>) {
    use std::collections::HashSet;

    for (label, caps) in [("can", &agent.can), ("cannot", &agent.cannot)] {
        let mut seen = HashSet::new();
        for cap in caps {
            let key = (cap.namespace.as_str(), cap.action.as_str());
            if !seen.insert(key) {
                diags.push(Diagnostic::warning(
                    "W003",
                    format!(
                        "duplicate capability '{}.{}' in {} list of agent '{}'",
                        cap.namespace, cap.action, label, agent.name
                    ),
                    cap.span.clone(),
                ));
            }
        }
    }
}

/// E005: duplicate workflow names.
fn check_duplicate_workflow_names(file: &ReinFile, diags: &mut Vec<Diagnostic>) {
    use std::collections::HashMap;
    let mut seen: HashMap<&str, usize> = HashMap::new();
    for workflow in &file.workflows {
        if let Some(&first_start) = seen.get(workflow.name.as_str()) {
            diags.push(Diagnostic::error(
                "E005",
                format!(
                    "duplicate workflow name '{}': first defined at {first_start}",
                    workflow.name
                ),
                workflow.span.clone(),
            ));
        } else {
            seen.insert(&workflow.name, workflow.span.start);
        }
    }
}

/// E006: workflow stage references a non-existent agent.
fn check_workflow_stages_reference_agents(
    file: &ReinFile,
    workflow: &crate::ast::WorkflowDef,
    diags: &mut Vec<Diagnostic>,
) {
    use std::collections::HashSet;
    let agent_names: HashSet<&str> = file.agents.iter().map(|a| a.name.as_str()).collect();
    for stage in &workflow.stages {
        if !agent_names.contains(stage.agent.as_str()) {
            diags.push(Diagnostic::error(
                "E006",
                format!(
                    "stage '{}' in workflow '{}' references unknown agent '{}'",
                    stage.name, workflow.name, stage.agent
                ),
                stage.span.clone(),
            ));
        }
    }
}

/// W004: duplicate stage names in a workflow.
fn check_duplicate_stages(workflow: &crate::ast::WorkflowDef, diags: &mut Vec<Diagnostic>) {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    for stage in &workflow.stages {
        if !seen.insert(stage.name.as_str()) {
            diags.push(Diagnostic::warning(
                "W004",
                format!(
                    "duplicate stage '{}' in workflow '{}'",
                    stage.name, workflow.name
                ),
                stage.span.clone(),
            ));
        }
    }
}

/// W001: agent has no `model` field.
fn check_model_present(agent: &AgentDef, diags: &mut Vec<Diagnostic>) {
    if agent.model.is_none() {
        diags.push(Diagnostic::warning(
            "W001",
            format!("agent '{}' has no `model` field", agent.name),
            agent.span.clone(),
        ));
    }
}

/// E008: step references an agent that doesn't exist.
fn check_workflow_steps_reference_agents(
    file: &ReinFile,
    workflow: &crate::ast::WorkflowDef,
    diags: &mut Vec<Diagnostic>,
) {
    for step in &workflow.steps {
        if !file.agents.iter().any(|a| a.name == step.agent) {
            diags.push(Diagnostic::error(
                "E008",
                format!(
                    "step '{}' in workflow '{}' references unknown agent '{}'",
                    step.name, workflow.name, step.agent
                ),
                step.span.clone(),
            ));
        }
    }
}

/// E009: duplicate step names within a workflow.
fn check_duplicate_step_names(workflow: &crate::ast::WorkflowDef, diags: &mut Vec<Diagnostic>) {
    use std::collections::HashMap;
    let mut seen: HashMap<&str, &crate::ast::StepDef> = HashMap::new();
    for step in &workflow.steps {
        if let Some(first) = seen.get(step.name.as_str()) {
            diags.push(Diagnostic::error(
                "E009",
                format!(
                    "duplicate step name '{}' in workflow '{}': first defined at {}",
                    step.name, workflow.name, first.span.start
                ),
                step.span.clone(),
            ));
        } else {
            seen.insert(&step.name, step);
        }
    }
}

/// E010: step name collides with a stage name in the same workflow.
fn check_step_stage_name_collisions(
    workflow: &crate::ast::WorkflowDef,
    diags: &mut Vec<Diagnostic>,
) {
    use std::collections::HashMap;
    let stage_names: HashMap<&str, &crate::ast::Stage> = workflow
        .stages
        .iter()
        .map(|s| (s.name.as_str(), s))
        .collect();
    for step in &workflow.steps {
        if let Some(stage) = stage_names.get(step.name.as_str()) {
            diags.push(Diagnostic::error(
                "E010",
                format!(
                    "step '{}' in workflow '{}' collides with stage of the same name at {}",
                    step.name, workflow.name, stage.span.start
                ),
                step.span.clone(),
            ));
        }
    }
}

/// E007: two providers with the same name.
fn check_duplicate_provider_names(file: &ReinFile, diags: &mut Vec<Diagnostic>) {
    use std::collections::HashMap;
    let mut seen: HashMap<&str, &ProviderDef> = HashMap::new();
    for provider in &file.providers {
        if let Some(first) = seen.get(provider.name.as_str()) {
            diags.push(Diagnostic::error(
                "E007",
                format!(
                    "duplicate provider name '{}': first defined at {}",
                    provider.name, first.span.start
                ),
                provider.span.clone(),
            ));
        } else {
            seen.insert(&provider.name, provider);
        }
    }
}

/// W005: provider has no `key` field.
fn check_provider_key_present(provider: &ProviderDef, diags: &mut Vec<Diagnostic>) {
    if provider.key.is_none() {
        diags.push(Diagnostic::warning(
            "W005",
            format!("provider '{}' has no `key` field", provider.name),
            provider.span.clone(),
        ));
    }
}

#[cfg(test)]
mod tests;
