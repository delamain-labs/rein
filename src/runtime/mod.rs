pub mod budget;
pub mod engine;
pub mod executor;
pub mod interceptor;
pub mod permissions;
pub mod provider;
pub mod workflow;

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

impl RunTrace {
    /// Serialize to pretty-printed JSON.
    ///
    /// # Errors
    /// Returns a serialization error if the trace cannot be serialized.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Produce a compact human-readable summary.
    #[must_use]
    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        let mut turn = 0_usize;

        for event in &self.events {
            match event {
                RunEvent::LlmCall { model, input_tokens, output_tokens, cost_cents } => {
                    turn += 1;
                    lines.push(format!(
                        "[turn {turn}] LLM call ({model}): {input_tokens} in / {output_tokens} out, {cost_cents}¢"
                    ));
                }
                RunEvent::ToolCallAttempt { tool, allowed, reason } => {
                    let status = if *allowed { "✓" } else { "✗" };
                    let tool_name = format!("{}.{}", tool.namespace, tool.action);
                    let suffix = reason.as_ref().map_or(String::new(), |r| format!(" ({r})"));
                    lines.push(format!("  {status} tool: {tool_name}{suffix}"));
                }
                RunEvent::ToolCallResult { tool, result } => {
                    let status = if result.success { "ok" } else { "err" };
                    let tool_name = format!("{}.{}", tool.namespace, tool.action);
                    let preview: String = result.output.chars().take(80).collect();
                    lines.push(format!("  → {tool_name} [{status}]: {preview}"));
                }
                RunEvent::BudgetUpdate { spent_cents, limit_cents } => {
                    lines.push(format!("  budget: {spent_cents}¢ / {limit_cents}¢"));
                }
                RunEvent::RunComplete { total_cost_cents, total_tokens } => {
                    lines.push(format!(
                        "Done. {total_tokens} tokens, {total_cost_cents}¢ total."
                    ));
                }
            }
        }

        lines.join("\n")
    }
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
