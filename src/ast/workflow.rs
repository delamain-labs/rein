use serde::{Deserialize, Serialize};

use super::types::TypeExpr;
use super::Span;

/// A single condition in an `auto resolve when` block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AutoResolveCondition {
    /// A comparison: `confidence > 90%`.
    Comparison(WhenComparison),
    /// A membership check: `action is one of [order_status, tracking]`.
    IsOneOf {
        field: String,
        variants: Vec<String>,
    },
}

/// An `auto resolve when { ... }` block for autonomous agent actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoResolveBlock {
    pub conditions: Vec<AutoResolveCondition>,
    pub span: Span,
}

/// Backoff strategy for retry policies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Exponential,
    Linear,
    Fixed,
}

/// Action to take after retries are exhausted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FailureAction {
    /// Escalate to a human or higher-level agent.
    Escalate,
    /// Execute a named step.
    Step(String),
}

/// A retry policy: `on failure: retry N strategy then action`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub backoff: BackoffStrategy,
    pub then: FailureAction,
}

/// A comparison operator in a `when` expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompareOp {
    Lt,
    Gt,
    LtEq,
    GtEq,
}

/// A value in a `when` comparison (right-hand side).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WhenValue {
    /// A percentage like `70%`.
    Percent(String),
    /// A currency amount like `$50`.
    Currency { symbol: char, amount: u64 },
    /// A plain number.
    Number(String),
    /// An identifier reference.
    Ident(String),
}

/// A single comparison in a `when` expression: `field op value`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WhenComparison {
    pub field: String,
    pub op: CompareOp,
    pub value: WhenValue,
}

/// A `when` expression combining comparisons with boolean logic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WhenExpr {
    Comparison(WhenComparison),
    And(Vec<WhenExpr>),
    Or(Vec<WhenExpr>),
}

/// A `parallel { step a {...} step b {...} }` block for concurrent execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParallelBlock {
    pub steps: Vec<StepDef>,
    pub span: Span,
}

/// A single arm in a `route on` block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteArm {
    /// The match pattern: a literal value or `_` for wildcard/default.
    pub pattern: RoutePattern,
    /// The step to execute if this arm matches.
    pub step: StepDef,
    pub span: Span,
}

/// A match pattern in a route arm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RoutePattern {
    /// Match a specific value.
    Value(String),
    /// Wildcard: match anything not matched above.
    Wildcard,
}

/// A `route on <expr> { ... }` block for pattern-matched routing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteBlock {
    /// Dot-separated field path to match on (e.g. `classify.category`).
    pub field_path: String,
    /// Match arms in order.
    pub arms: Vec<RouteArm>,
    pub span: Span,
}

/// How a condition is evaluated against agent output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", content = "value", rename_all = "snake_case")]
pub enum ConditionMatcher {
    /// Exact value match (case-insensitive, word-boundary aware).
    Equals(String),
    /// Substring containment (case-insensitive).
    Contains(String),
    /// Regular expression match.
    Regex(String),
    /// JSON path extraction and comparison (`path=expected`).
    JsonPath { path: String, expected: String },
}

/// How a workflow stage routes to the next stage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RouteRule {
    /// Always go to the next stage in sequence.
    Next,
    /// Route based on a condition in the agent's output.
    Conditional {
        field: String,
        matcher: ConditionMatcher,
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

/// A `step <name> { agent: <name> goal: <text> }` definition within a workflow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepDef {
    pub name: String,
    /// The agent to execute for this step.
    pub agent: String,
    /// Natural language goal describing what the step should accomplish.
    pub goal: Option<String>,
    /// Output type constraints (e.g. `category: one of [billing, technical]`).
    pub output_constraints: Vec<(String, TypeExpr)>,
    /// Optional guard condition: `when: confidence < 70%`.
    pub when: Option<WhenExpr>,
    /// Optional retry policy: `on failure: retry 3 exponential then escalate`.
    pub on_failure: Option<RetryPolicy>,
    /// Optional fallback step executed when the primary step fails.
    pub fallback: Option<Box<StepDef>>,
    pub span: Span,
}

/// A `workflow <name> { ... }` definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowDef {
    pub name: String,
    /// What triggers this workflow (e.g. `incoming_ticket`).
    pub trigger: String,
    /// Ordered list of stages.
    pub stages: Vec<Stage>,
    /// Named step blocks (`step <name> { ... }`).
    pub steps: Vec<StepDef>,
    /// Route-on blocks for pattern-matched routing.
    pub route_blocks: Vec<RouteBlock>,
    /// Parallel execution blocks.
    pub parallel_blocks: Vec<ParallelBlock>,
    /// Auto resolve conditions.
    pub auto_resolve: Option<AutoResolveBlock>,
    /// Default execution mode.
    pub mode: ExecutionMode,
    pub span: Span,
}

impl WorkflowDef {
    /// Find a stage by name within this workflow.
    pub fn find_stage(&self, name: &str) -> Option<&Stage> {
        self.stages.iter().find(|s| s.name == name)
    }
}
