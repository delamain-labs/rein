use std::collections::HashMap;
use std::sync::Mutex;

use super::budget::{BudgetTracker, calculate_cost};
use super::circuit_breaker::CircuitBreaker;
use super::executor::{Secrets, ToolExecutor};
use super::guardrails::GuardrailEngine;
use super::interceptor::{InterceptResult, ToolInterceptor};
use super::otel_export::OtelMode;
use super::permissions::ToolRegistry;
use super::policy::PolicyEngine;
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

/// Callback for streaming output events during a run.
pub trait StreamCallback: Send + Sync {
    /// Called when a text chunk is received from the LLM.
    fn on_text(&self, text: &str);
    /// Called when a tool call is about to be executed.
    fn on_tool_call(&self, namespace: &str, action: &str);
    /// Called when the run completes.
    fn on_complete(&self);
}

/// A no-op stream callback (default).
pub struct NoopStream;

impl StreamCallback for NoopStream {
    fn on_text(&self, _text: &str) {}
    fn on_tool_call(&self, _namespace: &str, _action: &str) {}
    fn on_complete(&self) {}
}

/// A stream callback that prints to stdout.
pub struct StdoutStream;

impl StreamCallback for StdoutStream {
    fn on_text(&self, text: &str) {
        use std::io::Write;
        print!("{text}");
        let _ = std::io::stdout().flush();
    }
    fn on_tool_call(&self, namespace: &str, action: &str) {
        eprintln!("[tool] {namespace}.{action}");
    }
    fn on_complete(&self) {
        println!();
    }
}

/// Mutable state carried through the agent loop.
struct RunState {
    messages: Vec<Message>,
    events: Vec<RunEvent>,
    total_tokens: u64,
    total_cost_cents: u64,
    budget: Option<BudgetTracker>,
    /// Accumulated LLM cost (in cents) attributed to each capped tool.
    /// Key is `"namespace.action"`. Only populated for `CappedAt` tools.
    per_tool_spent: HashMap<String, u64>,
}

/// The agent execution engine. Orchestrates the LLM, tool call, result loop
/// while enforcing permissions and budget constraints from the `.rein` file.
pub struct AgentEngine<'a> {
    provider: &'a dyn Provider,
    executor: &'a dyn ToolExecutor,
    interceptor: ToolInterceptor<'a>,
    tool_defs: Vec<ToolDef>,
    config: RunConfig,
    stream: Box<dyn StreamCallback + 'a>,
    guardrails: GuardrailEngine,
    circuit_breaker: Option<Mutex<CircuitBreaker>>,
    policy_engine: Option<Mutex<PolicyEngine>>,
    otel_mode: OtelMode,
    /// Agent name embedded in OTEL spans (e.g. `rein.run.<agent>`).
    agent_name: Option<String>,
    /// Resolved secrets injected from `secrets { }` blocks. Never emitted to trace.
    secrets: Secrets,
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
            stream: Box::new(NoopStream),
            guardrails: GuardrailEngine::empty(),
            circuit_breaker: None,
            policy_engine: None,
            otel_mode: OtelMode::None,
            agent_name: None,
            secrets: Secrets::from(HashMap::new()),
        }
    }

    /// Set a stream callback for real-time output.
    #[must_use]
    pub fn with_stream(mut self, stream: Box<dyn StreamCallback + 'a>) -> Self {
        self.stream = stream;
        self
    }

    /// Attach a guardrail engine for output filtering.
    #[must_use]
    pub fn with_guardrails(mut self, guardrails: GuardrailEngine) -> Self {
        self.guardrails = guardrails;
        self
    }

    /// Attach a circuit breaker for failure protection.
    #[must_use]
    pub fn with_circuit_breaker(mut self, cb: CircuitBreaker) -> Self {
        self.circuit_breaker = Some(Mutex::new(cb));
        self
    }

    /// Attach a policy engine for tier-based promotion tracking.
    #[must_use]
    pub fn with_policy(mut self, policy: PolicyEngine) -> Self {
        self.policy_engine = Some(Mutex::new(policy));
        self
    }

    /// Set the OTEL export mode driven by an `observe` block or `--otel` flag.
    #[must_use]
    pub fn with_otel_mode(mut self, mode: OtelMode) -> Self {
        self.otel_mode = mode;
        self
    }

    /// Set the agent name used in OTEL span names (e.g. `rein.run.<name>`).
    #[must_use]
    pub fn with_agent_name(mut self, name: String) -> Self {
        self.agent_name = Some(name);
        self
    }

    /// Inject resolved secrets from `secrets { }` blocks.
    /// Values are never emitted to the trace or OTEL spans.
    #[must_use]
    pub fn with_secrets(mut self, secrets: HashMap<String, String>) -> Self {
        self.secrets = Secrets::from(secrets);
        self
    }

    /// Run the agent with the given user message.
    ///
    /// # Errors
    /// Returns `RunError` if the run fails (budget exceeded, provider error, etc.).
    pub async fn run(&self, user_message: &str) -> Result<RunResult, RunError> {
        let run_start = std::time::Instant::now();
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
            per_tool_spent: HashMap::new(),
        };

        if let Some(ref prompt) = self.config.system_prompt {
            state.messages.push(Message::system(prompt));
        }
        state.messages.push(Message::user(user_message));

        for _turn in 0..self.config.max_turns {
            // Circuit breaker check before LLM call.
            if let Some(ref cb_mutex) = self.circuit_breaker {
                let mut cb = cb_mutex.lock().expect("circuit breaker lock");
                if let Err(_reason) = cb.check() {
                    state.events.push(RunEvent::CircuitBreakerTripped {
                        name: cb.name().to_string(),
                        failures: 0,
                        threshold: 0,
                    });
                    return Err(RunError::CircuitBreakerOpen);
                }
            }

            let response = self
                .provider
                .chat(&state.messages, &self.tool_defs)
                .await
                .map_err(|_| {
                    if let Some(ref cb_mutex) = self.circuit_breaker {
                        cb_mutex
                            .lock()
                            .expect("circuit breaker lock")
                            .record_failure();
                    }
                    RunError::ProviderError
                })?;

            if let Some(ref cb_mutex) = self.circuit_breaker {
                cb_mutex
                    .lock()
                    .expect("circuit breaker lock")
                    .record_success();
            }

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

            self.evaluate_policy(&mut state);

            // Apply guardrails to LLM output.
            let content = if self.guardrails.is_empty() {
                response.content.clone()
            } else {
                let result = self.guardrails.apply(&response.content);
                for violation in &result.violations {
                    state.events.push(RunEvent::GuardrailTriggered {
                        rule: violation.rule_key.clone(),
                        action: format!("{:?}", violation.action),
                        blocked: result.blocked,
                    });
                }
                if result.blocked {
                    return Err(RunError::GuardrailBlocked);
                }
                result.output
            };

            // Stream the response text.
            if !content.is_empty() {
                self.stream.on_text(&content);
            }

            if response.tool_calls.is_empty() {
                self.stream.on_complete();
                let result = Self::finish(state, content);
                self.apply_otel_export(&result, run_start.elapsed());
                return Ok(result);
            }

            state.messages.push(Message::assistant(&response.content));
            // Stream tool call notifications
            for tc in &response.tool_calls {
                self.stream.on_tool_call(&tc.name, &tc.name);
            }
            self.process_tool_calls(&mut state, &response.tool_calls, cost)
                .await;
        }

        let result = Self::finish(state, "Max turns reached".to_string());
        self.apply_otel_export(&result, run_start.elapsed());
        Ok(result)
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

    /// Evaluate policy promotion conditions after a turn and emit events.
    fn evaluate_policy(&self, state: &mut RunState) {
        let Some(ref policy_mutex) = self.policy_engine else {
            return;
        };
        let mut policy = policy_mutex.lock().expect("policy engine lock");
        #[allow(clippy::cast_precision_loss)]
        let metrics = vec![
            ("tokens".to_string(), state.total_tokens as f64),
            ("cost".to_string(), state.total_cost_cents as f64),
        ];
        if let Some(ev) = policy.evaluate_promotion(&metrics) {
            state.events.push(RunEvent::PolicyPromotion {
                from_tier: ev.from_tier,
                to_tier: ev.to_tier,
            });
        }
    }

    /// Process all tool calls from an LLM response.
    ///
    /// `turn_cost` is the LLM cost (cents) for this turn and is attributed to
    /// each capped tool that executes in this turn. Caps are checked BEFORE
    /// execution: if a tool's accumulated attributed cost has already reached
    /// its declared cap, the call is denied and the agent is informed.
    async fn process_tool_calls(
        &self,
        state: &mut RunState,
        tool_calls: &[ToolCallRequest],
        turn_cost: u64,
    ) {
        // Count capped tools in this batch to distribute turn_cost evenly.
        let capped_count = tool_calls
            .iter()
            .filter(|tc| {
                let tool = ToolCall {
                    namespace: Self::extract_namespace(&tc.name),
                    action: Self::extract_action(&tc.name),
                    arguments: serde_json::Value::Null,
                };
                matches!(
                    self.interceptor.intercept(&tool),
                    InterceptResult::CappedAt { .. }
                )
            })
            .count()
            .max(1) as u64;

        for tc_req in tool_calls {
            let tool_call = ToolCall {
                namespace: Self::extract_namespace(&tc_req.name),
                action: Self::extract_action(&tc_req.name),
                arguments: tc_req.arguments.clone(),
            };

            let intercept = self.interceptor.intercept(&tool_call);

            match intercept {
                InterceptResult::Allowed => {
                    self.execute_allowed_tool(state, &tc_req.id, tool_call)
                        .await;
                }
                InterceptResult::CappedAt { max_cents, .. } => {
                    let key = format!("{}.{}", tool_call.namespace, tool_call.action);
                    let spent = state.per_tool_spent.get(&key).copied().unwrap_or(0);
                    if spent >= max_cents {
                        let reason = format!(
                            "monetary cap of {max_cents}¢ exceeded \
                             ({spent}¢ attributed to `{key}`)"
                        );
                        state.events.push(RunEvent::ToolCallAttempt {
                            tool: tool_call,
                            allowed: false,
                            reason: Some(reason.clone()),
                        });
                        state
                            .messages
                            .push(Message::tool(&tc_req.id, format!("Permission denied: {reason}")));
                    } else {
                        // Attribute this turn's LLM cost (divided evenly across capped tools).
                        *state.per_tool_spent.entry(key).or_insert(0) +=
                            turn_cost / capped_count;
                        self.execute_allowed_tool(state, &tc_req.id, tool_call)
                            .await;
                    }
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

        let ctx = super::executor::ToolCallContext {
            tool_call: &tool_call,
            secrets: &self.secrets,
        };
        match self.executor.execute(&ctx).await {
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

    /// Apply OTEL export after a run completes (side-effect only, never fails loudly).
    fn apply_otel_export(&self, result: &RunResult, duration: std::time::Duration) {
        let name = self.agent_name.as_deref().unwrap_or("agent");
        match &self.otel_mode {
            OtelMode::None => {}
            OtelMode::FileOnComplete => {
                Self::export_otel_to_file(result, duration, name);
            }
            OtelMode::StdoutOnComplete { metrics } => {
                Self::export_otel_to_stdout(result, duration, metrics, name);
            }
        }
    }

    /// Build a `StructuredTrace` with wall-clock timestamps from a completed run.
    ///
    /// Returns `(trace, completed_at)` so callers can reuse the captured timestamp
    /// (e.g. for a filename) without a second `Utc::now()` call that would drift.
    fn build_structured_trace(
        result: &RunResult,
        duration: std::time::Duration,
        agent_name: &str,
    ) -> (super::StructuredTrace, chrono::DateTime<chrono::Utc>) {
        let now = chrono::Utc::now();
        let started =
            now - chrono::Duration::from_std(duration).unwrap_or(chrono::Duration::zero());
        let trace = result.trace.to_structured(
            agent_name,
            &started.to_rfc3339(),
            &now.to_rfc3339(),
            duration.as_millis().try_into().unwrap_or(u64::MAX),
        );
        (trace, now)
    }

    /// Write OTLP JSON to a timestamped file.
    fn export_otel_to_file(result: &RunResult, duration: std::time::Duration, agent_name: &str) {
        let (structured, completed_at) = Self::build_structured_trace(result, duration, agent_name);
        match super::otel_export::to_otlp_json(&structured) {
            Ok(json) => {
                // Reuse the timestamp captured in build_structured_trace so the
                // filename matches the completed_at field inside the JSON.
                let ts = completed_at.format("%Y%m%d-%H%M%S");
                let path = format!("rein-trace-{ts}.json");
                match std::fs::write(&path, &json) {
                    Ok(()) => eprintln!("OTLP trace written to {path}"),
                    Err(e) => eprintln!("Failed to write OTLP trace: {e}"),
                }
            }
            Err(e) => eprintln!("Failed to serialize OTLP trace: {e}"),
        }
    }

    /// Print filtered OTLP JSON spans to stdout.
    fn export_otel_to_stdout(
        result: &RunResult,
        duration: std::time::Duration,
        metrics: &[String],
        agent_name: &str,
    ) {
        use super::otel_export::to_otlp;
        let (mut structured, _completed_at) =
            Self::build_structured_trace(result, duration, agent_name);
        if !metrics.is_empty() {
            structured
                .events
                .retain(|te| event_matches_metrics(&te.event, metrics));
        }
        let resource_spans = to_otlp(&structured);
        match serde_json::to_string_pretty(&resource_spans) {
            Ok(json) => println!("{json}"),
            Err(e) => eprintln!("Failed to serialize OTLP trace: {e}"),
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

/// Returns `true` if the event contributes to any of the requested metric categories.
///
/// Metric names:
/// - `"cost"` → `LlmCall`, `BudgetUpdate`, `RunComplete`
/// - `"tool_calls"` → `ToolCallAttempt`, `ToolCallResult`
/// - `"latency"` → `LlmCall`
/// - `"guardrails"` → `GuardrailTriggered`
fn event_matches_metrics(event: &RunEvent, metrics: &[String]) -> bool {
    use super::RunEvent;
    metrics.iter().any(|m| match m.as_str() {
        "cost" => matches!(
            event,
            RunEvent::LlmCall { .. } | RunEvent::BudgetUpdate { .. } | RunEvent::RunComplete { .. }
        ),
        "tool_calls" => matches!(
            event,
            RunEvent::ToolCallAttempt { .. } | RunEvent::ToolCallResult { .. }
        ),
        "latency" => matches!(event, RunEvent::LlmCall { .. }),
        "guardrails" => matches!(event, RunEvent::GuardrailTriggered { .. }),
        _ => false,
    })
}
