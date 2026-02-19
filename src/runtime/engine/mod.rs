use super::budget::{BudgetTracker, calculate_cost};
use super::executor::ToolExecutor;
use super::interceptor::{InterceptResult, ToolInterceptor};
use super::permissions::ToolRegistry;
use super::provider::{Message, Provider, ToolCallRequest, ToolDef};
use super::{RunError, RunEvent, RunTrace, ToolCall};

#[cfg(test)]
mod tests;

/// Configuration for an agent run.
#[derive(Debug, Clone)]
pub struct RunConfig {
    /// System prompt to prepend.
    pub system_prompt: Option<String>,
    /// Maximum number of LLM round-trips before stopping.
    pub max_turns: usize,
    /// Budget limit in cents (0 = no budget).
    pub budget_cents: u64,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            system_prompt: None,
            max_turns: 10,
            budget_cents: 0,
        }
    }
}

/// The result of a completed agent run.
#[derive(Debug)]
pub struct RunResult {
    /// Final assistant response text.
    pub response: String,
    /// Full trace of events.
    pub trace: RunTrace,
    /// Total tokens used.
    pub total_tokens: u64,
    /// Total cost in cents.
    pub total_cost_cents: u64,
}

/// Mutable state carried through the agent loop.
struct RunState {
    messages: Vec<Message>,
    events: Vec<RunEvent>,
    total_tokens: u64,
    total_cost_cents: u64,
    budget: Option<BudgetTracker>,
}

/// The agent execution engine. Orchestrates the LLM, tool call, result loop
/// while enforcing permissions and budget constraints from the `.rein` file.
pub struct AgentEngine<'a> {
    provider: &'a dyn Provider,
    executor: &'a dyn ToolExecutor,
    interceptor: ToolInterceptor<'a>,
    tool_defs: Vec<ToolDef>,
    config: RunConfig,
}

impl<'a> AgentEngine<'a> {
    /// Create a new engine.
    #[must_use]
    pub fn new(
        provider: &'a dyn Provider,
        executor: &'a dyn ToolExecutor,
        registry: &'a ToolRegistry,
        tool_defs: Vec<ToolDef>,
        config: RunConfig,
    ) -> Self {
        Self {
            provider,
            executor,
            interceptor: ToolInterceptor::new(registry),
            tool_defs,
            config,
        }
    }

    /// Run the agent with the given user message.
    ///
    /// # Errors
    /// Returns `RunError` if the run fails (budget exceeded, provider error, etc.).
    pub async fn run(&self, user_message: &str) -> Result<RunResult, RunError> {
        let mut state = RunState {
            messages: Vec::new(),
            events: Vec::new(),
            total_tokens: 0,
            total_cost_cents: 0,
            budget: if self.config.budget_cents > 0 {
                Some(BudgetTracker::new(self.config.budget_cents))
            } else {
                None
            },
        };

        if let Some(ref prompt) = self.config.system_prompt {
            state.messages.push(Message::system(prompt));
        }
        state.messages.push(Message::user(user_message));

        for _turn in 0..self.config.max_turns {
            let response = self
                .provider
                .chat(&state.messages, &self.tool_defs)
                .await
                .map_err(|_| RunError::ProviderError)?;

            let cost = calculate_cost(&response.model, &response.usage);
            state.total_tokens += response.usage.input_tokens + response.usage.output_tokens;
            state.total_cost_cents += cost;

            state.events.push(RunEvent::LlmCall {
                model: response.model.clone(),
                input_tokens: response.usage.input_tokens,
                output_tokens: response.usage.output_tokens,
                cost_cents: cost,
            });

            self.check_budget(&mut state, cost)?;

            if response.tool_calls.is_empty() {
                return Ok(Self::finish(state, response.content));
            }

            state.messages.push(Message::assistant(&response.content));
            self.process_tool_calls(&mut state, &response.tool_calls)
                .await;
        }

        Ok(Self::finish(state, "Max turns reached".to_string()))
    }

    /// Check budget and record the cost. Returns `Err` if exceeded.
    fn check_budget(&self, state: &mut RunState, cost: u64) -> Result<(), RunError> {
        if let Some(ref mut tracker) = state.budget {
            if tracker.record_usage(cost).is_err() {
                state.events.push(RunEvent::BudgetUpdate {
                    spent_cents: state.total_cost_cents,
                    limit_cents: self.config.budget_cents,
                });
                return Err(RunError::BudgetExceeded);
            }
            state.events.push(RunEvent::BudgetUpdate {
                spent_cents: tracker.spent_cents(),
                limit_cents: tracker.limit_cents(),
            });
        }
        Ok(())
    }

    /// Process all tool calls from an LLM response.
    async fn process_tool_calls(&self, state: &mut RunState, tool_calls: &[ToolCallRequest]) {
        for tc_req in tool_calls {
            let tool_call = ToolCall {
                namespace: Self::extract_namespace(&tc_req.name),
                action: Self::extract_action(&tc_req.name),
                arguments: tc_req.arguments.clone(),
            };

            let intercept = self.interceptor.intercept(&tool_call);

            match intercept {
                InterceptResult::Allowed | InterceptResult::CappedAt { .. } => {
                    self.execute_allowed_tool(state, &tc_req.id, tool_call)
                        .await;
                }
                InterceptResult::Denied { reason } => {
                    state.events.push(RunEvent::ToolCallAttempt {
                        tool: tool_call,
                        allowed: false,
                        reason: Some(reason.clone()),
                    });
                    state.messages.push(Message::tool(
                        &tc_req.id,
                        format!("Permission denied: {reason}"),
                    ));
                }
            }
        }
    }

    /// Execute an allowed tool call and record the results.
    async fn execute_allowed_tool(&self, state: &mut RunState, call_id: &str, tool_call: ToolCall) {
        state.events.push(RunEvent::ToolCallAttempt {
            tool: tool_call.clone(),
            allowed: true,
            reason: None,
        });

        match self.executor.execute(&tool_call).await {
            Ok(output) => {
                state.messages.push(Message::tool(call_id, &output.output));
                state.events.push(RunEvent::ToolCallResult {
                    tool: tool_call,
                    result: super::ToolResult {
                        success: output.success,
                        output: output.output,
                    },
                });
            }
            Err(e) => {
                let error_msg = e.to_string();
                state.messages.push(Message::tool(call_id, &error_msg));
                state.events.push(RunEvent::ToolCallResult {
                    tool: tool_call,
                    result: super::ToolResult {
                        success: false,
                        output: error_msg,
                    },
                });
            }
        }
    }

    /// Build the final `RunResult`.
    fn finish(state: RunState, response: String) -> RunResult {
        let mut events = state.events;
        events.push(RunEvent::RunComplete {
            total_cost_cents: state.total_cost_cents,
            total_tokens: state.total_tokens,
        });
        RunResult {
            response,
            trace: RunTrace { events },
            total_tokens: state.total_tokens,
            total_cost_cents: state.total_cost_cents,
        }
    }

    /// Extract namespace from a tool name like `zendesk.read_ticket`.
    fn extract_namespace(name: &str) -> String {
        name.find('.').map_or_else(
            || {
                name.find('_')
                    .map_or_else(|| name.to_string(), |i| name[..i].to_string())
            },
            |i| name[..i].to_string(),
        )
    }

    /// Extract action from a tool name like `zendesk.read_ticket`.
    fn extract_action(name: &str) -> String {
        name.find('.').map_or_else(
            || {
                name.find('_')
                    .map_or_else(|| name.to_string(), |i| name[i + 1..].to_string())
            },
            |i| name[i + 1..].to_string(),
        )
    }
}
