pub mod budget;
pub mod engine;
pub mod executor;
pub mod interceptor;
pub mod permissions;
pub mod provider;

use serde::{Deserialize, Serialize};

/// A tool invocation requested by the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub namespace: String,
    pub action: String,
    pub arguments: serde_json::Value,
}

/// The outcome of executing a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
}

/// A discrete event that occurs during an agent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunEvent {
    LlmCall {
        model: String,
        input_tokens: u64,
        output_tokens: u64,
        cost_cents: u64,
    },
    ToolCallAttempt {
        tool: ToolCall,
        allowed: bool,
        reason: Option<String>,
    },
    ToolCallResult {
        tool: ToolCall,
        result: ToolResult,
    },
    BudgetUpdate {
        spent_cents: u64,
        limit_cents: u64,
    },
    RunComplete {
        total_cost_cents: u64,
        total_tokens: u64,
    },
}

/// An ordered log of all events that occurred during a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunTrace {
    pub events: Vec<RunEvent>,
}

/// Errors that can occur during an agent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunError {
    BudgetExceeded,
    PermissionDenied,
    ProviderError,
    ConfigError,
}

#[cfg(test)]
mod tests;
