use serde::{Deserialize, Serialize};

/// Byte-offset span in the source file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// A value expression used in configuration fields.
///
/// Supports literal strings and function calls like `env("VAR_NAME")`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ValueExpr {
    /// A plain string or identifier value.
    Literal(String),
    /// An environment variable reference: `env("VAR_NAME")`.
    EnvRef { var_name: String, span: Span },
}

/// Error from resolving a `ValueExpr`.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolveError {
    /// An environment variable was not found.
    EnvVarNotSet(String),
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EnvVarNotSet(var) => write!(f, "environment variable '{var}' is not set"),
        }
    }
}

impl std::error::Error for ResolveError {}

impl ValueExpr {
    /// Resolve to a plain string value using the provided env lookup function.
    ///
    /// For `Literal`, returns the string directly. For `EnvRef`, calls
    /// `env_lookup` with the variable name.
    pub fn resolve_with<F>(&self, env_lookup: F) -> Result<String, ResolveError>
    where
        F: Fn(&str) -> Option<String>,
    {
        match self {
            Self::Literal(s) => Ok(s.clone()),
            Self::EnvRef { var_name, .. } => env_lookup(var_name)
                .ok_or_else(|| ResolveError::EnvVarNotSet(var_name.clone())),
        }
    }

    /// Resolve using `std::env::var`. Convenience wrapper around `resolve_with`.
    pub fn resolve(&self) -> Result<String, ResolveError> {
        self.resolve_with(|name| std::env::var(name).ok())
    }

    /// Return the literal string value if this is a `Literal`.
    pub fn as_literal(&self) -> Option<&str> {
        match self {
            Self::Literal(s) => Some(s),
            Self::EnvRef { .. } => None,
        }
    }

    /// Return a display-friendly string for this value.
    /// For `Literal`, returns the string. For `EnvRef`, returns the var name.
    pub fn display_value(&self) -> &str {
        match self {
            Self::Literal(s) => s,
            Self::EnvRef { var_name, .. } => var_name,
        }
    }
}

/// A monetary cap constraint on a capability (`up to $<amount>`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Constraint {
    MonetaryCap { amount: u64, currency: String },
}

/// A single tool capability, e.g. `zendesk.read_ticket` or `zendesk.refund up to $50`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Capability {
    pub namespace: String,
    pub action: String,
    pub constraint: Option<Constraint>,
    pub span: Span,
}

/// A spending budget, e.g. `budget: $0.03 per ticket`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Budget {
    pub amount: u64,
    pub currency: String,
    pub unit: String,
    pub span: Span,
}

/// A single `agent <name> { ... }` definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentDef {
    pub name: String,
    pub model: Option<ValueExpr>,
    pub can: Vec<Capability>,
    pub cannot: Vec<Capability>,
    pub budget: Option<Budget>,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Workflow types
// ---------------------------------------------------------------------------

/// How a workflow stage routes to the next stage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RouteRule {
    /// Always go to the next stage in sequence.
    Next,
    /// Route based on a condition in the agent's output.
    Conditional {
        field: String,
        equals: String,
        then_stage: String,
        else_stage: Option<String>,
    },
}

/// A single stage in a workflow pipeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stage {
    /// Name of this stage (used for routing references).
    pub name: String,
    /// Name of the agent to run at this stage.
    pub agent: String,
    /// How to route after this stage completes.
    pub route: RouteRule,
    pub span: Span,
}

/// Execution mode for a group of stages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    Sequential,
    Parallel,
}

/// A `workflow <name> { ... }` definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowDef {
    pub name: String,
    /// What triggers this workflow (e.g. `incoming_ticket`).
    pub trigger: String,
    /// Ordered list of stages.
    pub stages: Vec<Stage>,
    /// Default execution mode.
    pub mode: ExecutionMode,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Top-level file
// ---------------------------------------------------------------------------

/// Top-level parsed file — agent and workflow definitions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReinFile {
    pub agents: Vec<AgentDef>,
    pub workflows: Vec<WorkflowDef>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_span() -> Span {
        Span::new(0, 1)
    }

    #[test]
    fn span_serializes_to_json() {
        let span = Span::new(10, 20);
        let json = serde_json::to_string(&span).unwrap();
        assert_eq!(json, r#"{"start":10,"end":20}"#);
    }

    #[test]
    fn constraint_monetary_cap_serializes() {
        let c = Constraint::MonetaryCap {
            amount: 5000,
            currency: "USD".to_string(),
        };
        let json = serde_json::to_value(&c).unwrap();
        assert_eq!(json["type"], "MonetaryCap");
        assert_eq!(json["amount"], 5000);
        assert_eq!(json["currency"], "USD");
    }

    #[test]
    fn capability_with_constraint_serializes() {
        let cap = Capability {
            namespace: "zendesk".to_string(),
            action: "refund".to_string(),
            constraint: Some(Constraint::MonetaryCap {
                amount: 5000,
                currency: "USD".to_string(),
            }),
            span: dummy_span(),
        };
        let json = serde_json::to_value(&cap).unwrap();
        assert_eq!(json["namespace"], "zendesk");
        assert_eq!(json["action"], "refund");
        assert_eq!(json["constraint"]["amount"], 5000);
    }

    #[test]
    fn capability_without_constraint_serializes() {
        let cap = Capability {
            namespace: "zendesk".to_string(),
            action: "read_ticket".to_string(),
            constraint: None,
            span: dummy_span(),
        };
        let json = serde_json::to_value(&cap).unwrap();
        assert_eq!(json["constraint"], serde_json::Value::Null);
    }

    #[test]
    fn budget_serializes() {
        let b = Budget {
            amount: 3,
            currency: "USD".to_string(),
            unit: "ticket".to_string(),
            span: dummy_span(),
        };
        let json = serde_json::to_value(&b).unwrap();
        assert_eq!(json["amount"], 3);
        assert_eq!(json["currency"], "USD");
        assert_eq!(json["unit"], "ticket");
    }

    #[test]
    fn agent_def_full_serializes() {
        let agent = AgentDef {
            name: "support_triage".to_string(),
            model: Some(ValueExpr::Literal("anthropic".into())),
            can: vec![Capability {
                namespace: "zendesk".to_string(),
                action: "read_ticket".to_string(),
                constraint: None,
                span: dummy_span(),
            }],
            cannot: vec![Capability {
                namespace: "zendesk".to_string(),
                action: "delete_ticket".to_string(),
                constraint: None,
                span: dummy_span(),
            }],
            budget: Some(Budget {
                amount: 3,
                currency: "USD".to_string(),
                unit: "ticket".to_string(),
                span: dummy_span(),
            }),
            span: dummy_span(),
        };
        let json = serde_json::to_value(&agent).unwrap();
        assert_eq!(json["name"], "support_triage");
        assert_eq!(json["model"], "anthropic");
        assert_eq!(json["can"].as_array().unwrap().len(), 1);
        assert_eq!(json["cannot"].as_array().unwrap().len(), 1);
        assert_eq!(json["budget"]["unit"], "ticket");
    }

    #[test]
    fn rein_file_roundtrips_via_json() {
        let file = ReinFile {
            agents: vec![AgentDef {
                name: "bot".to_string(),
                model: None,
                can: vec![],
                cannot: vec![],
                budget: None,
                span: dummy_span(),
            }],
            workflows: vec![],
        };
        let json = serde_json::to_string(&file).unwrap();
        let decoded: ReinFile = serde_json::from_str(&json).unwrap();
        assert_eq!(file, decoded);
    }

    #[test]
    fn agent_def_minimal_model_none() {
        let agent = AgentDef {
            name: "minimal".to_string(),
            model: None,
            can: vec![],
            cannot: vec![],
            budget: None,
            span: dummy_span(),
        };
        let json = serde_json::to_value(&agent).unwrap();
        assert_eq!(json["model"], serde_json::Value::Null);
        assert!(json["can"].as_array().unwrap().is_empty());
    }

    // ── Workflow types ─────────────────────────────────────────────────

    #[test]
    fn stage_serializes() {
        let stage = Stage {
            name: "triage".to_string(),
            agent: "support_triage".to_string(),
            route: RouteRule::Next,
            span: dummy_span(),
        };
        let json = serde_json::to_value(&stage).unwrap();
        assert_eq!(json["name"], "triage");
        assert_eq!(json["agent"], "support_triage");
        assert_eq!(json["route"]["type"], "next");
    }

    #[test]
    fn conditional_route_serializes() {
        let route = RouteRule::Conditional {
            field: "sentiment".to_string(),
            equals: "negative".to_string(),
            then_stage: "escalate".to_string(),
            else_stage: Some("respond".to_string()),
        };
        let json = serde_json::to_value(&route).unwrap();
        assert_eq!(json["type"], "conditional");
        assert_eq!(json["field"], "sentiment");
        assert_eq!(json["then_stage"], "escalate");
    }

    #[test]
    fn workflow_def_serializes() {
        let workflow = WorkflowDef {
            name: "support_pipeline".to_string(),
            trigger: "incoming_ticket".to_string(),
            stages: vec![
                Stage {
                    name: "triage".to_string(),
                    agent: "support_triage".to_string(),
                    route: RouteRule::Next,
                    span: dummy_span(),
                },
                Stage {
                    name: "respond".to_string(),
                    agent: "responder".to_string(),
                    route: RouteRule::Next,
                    span: dummy_span(),
                },
            ],
            mode: ExecutionMode::Sequential,
            span: dummy_span(),
        };
        let json = serde_json::to_value(&workflow).unwrap();
        assert_eq!(json["name"], "support_pipeline");
        assert_eq!(json["trigger"], "incoming_ticket");
        assert_eq!(json["stages"].as_array().unwrap().len(), 2);
        assert_eq!(json["mode"], "sequential");
    }

    #[test]
    fn workflow_roundtrips_via_json() {
        let workflow = WorkflowDef {
            name: "test".to_string(),
            trigger: "event".to_string(),
            stages: vec![],
            mode: ExecutionMode::Parallel,
            span: dummy_span(),
        };
        let json = serde_json::to_string(&workflow).unwrap();
        let decoded: WorkflowDef = serde_json::from_str(&json).unwrap();
        assert_eq!(workflow, decoded);
    }

    #[test]
    fn rein_file_with_workflows_roundtrips() {
        let file = ReinFile {
            agents: vec![],
            workflows: vec![WorkflowDef {
                name: "pipeline".to_string(),
                trigger: "event".to_string(),
                stages: vec![],
                mode: ExecutionMode::Sequential,
                span: dummy_span(),
            }],
        };
        let json = serde_json::to_string(&file).unwrap();
        let decoded: ReinFile = serde_json::from_str(&json).unwrap();
        assert_eq!(file, decoded);
    }
}
