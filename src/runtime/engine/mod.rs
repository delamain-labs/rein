use std::collections::HashMap;
use std::sync::Mutex;

use super::budget::{BudgetExceeded, BudgetTracker, calculate_cost};
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

/// Error variants returned by `AgentEngine::call_provider_with_timeout`.
///
/// Kept module-private — callers inside `run()` translate these into the
/// public `RunError` variants before returning to the outside world.
enum CallError {
    /// The provider call exceeded `stage_timeout_secs`. Carries the timeout
    /// duration so the caller can emit an accurate `StageTimeout` event.
    Timeout { secs: u64 },
    /// The provider returned an error (non-timeout failure).
    Provider,
}

/// Configuration for an agent run.
#[derive(Debug, Clone)]
pub struct RunConfig {
    /// System prompt to prepend.
    pub system_prompt: Option<String>,
    /// Maximum number of LLM round-trips before stopping.
    pub max_turns: usize,
    /// Budget limit in cents (0 = no budget).
    pub budget_cents: u64,
    /// Per-LLM-call timeout in seconds. `None` means no timeout.
    pub stage_timeout_secs: Option<u64>,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            system_prompt: None,
            max_turns: 10,
            budget_cents: 0,
            stage_timeout_secs: None,
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
    /// Wall-clock offsets (ms from `start`) captured at each event push.
    /// Always the same length as `events`.
    event_timestamps_ms: Vec<u64>,
    /// Monotonic start time for computing per-event offsets.
    start: std::time::Instant,
    total_tokens: u64,
    total_cost_cents: u64,
    budget: Option<BudgetTracker>,
    /// Accumulated LLM cost (in cents) attributed to each capped tool.
    /// Key is `"namespace.action"`. Only populated for `CappedAt` tools.
    per_tool_spent: HashMap<String, u64>,
}

impl RunState {
    /// Push an event and record its real wall-clock offset from run start.
    fn push(&mut self, event: RunEvent) {
        // Saturate at u64::MAX (~585 million years). We clamp before casting so
        // the truncation is intentional and the value is always representable.
        #[allow(clippy::cast_possible_truncation)]
        let elapsed_ms = self.start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        self.event_timestamps_ms.push(elapsed_ms);
        self.events.push(event);
    }
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

    /// Validates `RunConfig` before a run starts.
    ///
    /// # Errors
    /// Returns `Err(RunError::ConfigError)` if `stage_timeout_secs` is `Some(0)`
    /// (a zero-second timeout is not a valid no-timeout sentinel; use `None`).
    fn validate_config(&self) -> Result<(), RunError> {
        if self.config.stage_timeout_secs == Some(0) {
            return Err(RunError::ConfigError);
        }
        Ok(())
    }

    /// Run the agent with the given user message.
    ///
    /// # Errors
    /// Returns `RunError` if the run fails. Possible variants include:
    /// - `RunError::ConfigError` — invalid `RunConfig` (e.g. `stage_timeout_secs = Some(0)`)
    /// - `RunError::BudgetExceeded` — accumulated cost exceeded `budget_cents`
    /// - `RunError::Timeout` — provider call exceeded `stage_timeout_secs`
    /// - `RunError::ProviderError` — provider returned a non-timeout failure
    /// - `RunError::CircuitBreakerOpen` — circuit breaker is open
    /// - `RunError::GuardrailBlocked` — a guardrail condition was triggered
    #[allow(clippy::too_many_lines)] // tracked in #460
    pub async fn run(&self, user_message: &str) -> Result<RunResult, RunError> {
        self.validate_config()?;
        let run_start = std::time::Instant::now();
        let mut state = RunState {
            messages: Vec::new(),
            events: Vec::new(),
            event_timestamps_ms: Vec::new(),
            start: run_start,
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

        for turn in 0..self.config.max_turns {
            // Circuit breaker check before LLM call.
            if let Some(ref cb_mutex) = self.circuit_breaker {
                let mut cb = cb_mutex.lock().expect("circuit breaker lock");
                if let Err(_reason) = cb.check() {
                    state.push(RunEvent::CircuitBreakerTripped {
                        name: cb.name().to_string(),
                        failures: cb.failure_count(),
                        threshold: cb.threshold(),
                    });
                    let partial = RunTrace::from_events(std::mem::take(&mut state.events));
                    self.export_partial(
                        &partial,
                        state.total_tokens,
                        state.total_cost_cents,
                        run_start.elapsed(),
                    );
                    return Err(RunError::CircuitBreakerOpen {
                        partial_trace: partial,
                    });
                }
            }

            let response = match self.call_provider_with_timeout(&state.messages).await {
                Ok(r) => r,
                Err(CallError::Timeout { secs }) => {
                    state.push(RunEvent::StageTimeout {
                        turn,
                        timeout_secs: secs,
                    });
                    let partial = RunTrace::from_events(std::mem::take(&mut state.events));
                    self.export_partial(
                        &partial,
                        state.total_tokens,
                        state.total_cost_cents,
                        run_start.elapsed(),
                    );
                    return Err(RunError::Timeout {
                        partial_trace: partial,
                    });
                }
                Err(CallError::Provider) => return Err(RunError::ProviderError),
            };
            self.record_cb_success();

            let cost = calculate_cost(&response.model, &response.usage);
            state.total_tokens += response.usage.input_tokens + response.usage.output_tokens;
            state.total_cost_cents += cost;

            state.push(RunEvent::LlmCall {
                model: response.model.clone(),
                input_tokens: response.usage.input_tokens,
                output_tokens: response.usage.output_tokens,
                cost_cents: cost,
            });

            if let Err(e) = Self::check_budget(&mut state, cost) {
                // Export partial OTEL trace on budget exhaustion so
                // observability hooks capture the partial run (#479).
                if let RunError::BudgetExceeded { ref partial_trace } = e {
                    self.export_partial(
                        partial_trace,
                        state.total_tokens,
                        state.total_cost_cents,
                        run_start.elapsed(),
                    );
                }
                return Err(e);
            }

            self.evaluate_policy(&mut state);

            // Apply guardrails to LLM output.
            let content = if self.guardrails.is_empty() {
                response.content.clone()
            } else {
                let result = self.guardrails.apply(&response.content);
                for violation in &result.violations {
                    state.push(RunEvent::GuardrailTriggered {
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

    /// Call the provider for one turn, applying the stage timeout if configured.
    ///
    /// Records a circuit-breaker failure on both timeout and provider error.
    /// Returns the `ChatResponse` on success, or a `CallError` on failure.
    async fn call_provider_with_timeout(
        &self,
        messages: &[Message],
    ) -> Result<super::provider::ChatResponse, CallError> {
        let chat_future = self.provider.chat(messages, &self.tool_defs);
        let result = if let Some(secs) = self.config.stage_timeout_secs.filter(|&s| s > 0) {
            match tokio::time::timeout(std::time::Duration::from_secs(secs), chat_future).await {
                Ok(r) => r,
                Err(_elapsed) => {
                    // Unresponsive provider counts as a failure for the
                    // circuit breaker, same as a provider error.
                    self.record_cb_failure();
                    return Err(CallError::Timeout { secs });
                }
            }
        } else {
            chat_future.await
        };
        result.map_err(|_| {
            self.record_cb_failure();
            CallError::Provider
        })
    }

    /// Record a circuit-breaker failure (no-op if no circuit breaker is wired).
    fn record_cb_failure(&self) {
        if let Some(ref cb_mutex) = self.circuit_breaker {
            cb_mutex
                .lock()
                .expect("circuit breaker lock")
                .record_failure();
        }
    }

    /// Record a circuit-breaker success (no-op if no circuit breaker is wired).
    fn record_cb_success(&self) {
        if let Some(ref cb_mutex) = self.circuit_breaker {
            cb_mutex
                .lock()
                .expect("circuit breaker lock")
                .record_success();
        }
    }

    /// Check budget and record the cost. Returns `Err` if exceeded.
    fn check_budget(state: &mut RunState, cost: u64) -> Result<(), RunError> {
        /// Outcome of a single budget check, carrying the event to emit.
        enum BudgetOutcome {
            /// Budget updated and still within limit.
            WithinLimit(RunEvent),
            /// Budget exceeded; emit this event then abort.
            Exceeded(RunEvent),
            /// No budget configured; nothing to do.
            NoBudget,
        }

        let outcome = if let Some(ref mut tracker) = state.budget {
            match tracker.record_usage(cost) {
                Err(BudgetExceeded {
                    spent_cents,
                    limit_cents,
                }) => BudgetOutcome::Exceeded(RunEvent::BudgetUpdate {
                    spent_cents,
                    limit_cents,
                }),
                Ok(()) => BudgetOutcome::WithinLimit(RunEvent::BudgetUpdate {
                    spent_cents: tracker.spent_cents(),
                    limit_cents: tracker.limit_cents(),
                }),
            }
        } else {
            BudgetOutcome::NoBudget
        };
        match outcome {
            BudgetOutcome::Exceeded(event) => {
                state.push(event);
                let partial = RunTrace::from_events(std::mem::take(&mut state.events));
                return Err(RunError::BudgetExceeded {
                    partial_trace: partial,
                });
            }
            BudgetOutcome::WithinLimit(event) => state.push(event),
            BudgetOutcome::NoBudget => {}
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
            state.push(RunEvent::PolicyPromotion {
                from_tier: ev.from_tier,
                to_tier: ev.to_tier,
            });
        }
    }

    /// Process all tool calls from an LLM response.
    ///
    /// `turn_cost` is the LLM cost (cents) for this turn. Each capped tool that
    /// is **allowed to execute** (i.e., has not yet reached its cap) gets an equal
    /// share of `turn_cost` attributed to it. Intercept results are computed once
    /// per tool call and reused for both cap-checking and dispatch.
    async fn process_tool_calls(
        &self,
        state: &mut RunState,
        tool_calls: &[ToolCallRequest],
        turn_cost: u64,
    ) {
        // Single pass: resolve every tool call and its intercept result.
        let resolved: Vec<(&ToolCallRequest, ToolCall, InterceptResult)> = tool_calls
            .iter()
            .map(|tc_req| {
                let tool_call = ToolCall {
                    namespace: Self::extract_namespace(&tc_req.name),
                    action: Self::extract_action(&tc_req.name),
                    arguments: tc_req.arguments.clone(),
                };
                let intercept = self.interceptor.intercept(&tool_call);
                (tc_req, tool_call, intercept)
            })
            .collect();

        // Count capped tools that are within their cap and will actually execute.
        // Denied-by-cap tools are excluded so cost is not under-attributed to
        // the tools that do execute.
        let executing_capped_count = resolved
            .iter()
            .filter(|(_, tool_call, intercept)| {
                if let InterceptResult::CappedAt { max_cents, .. } = intercept {
                    let key = format!("{}.{}", tool_call.namespace, tool_call.action);
                    let spent = state.per_tool_spent.get(&key).copied().unwrap_or(0);
                    spent < *max_cents
                } else {
                    false
                }
            })
            .count()
            .max(1) as u64;

        for (tc_req, tool_call, intercept) in resolved {
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
                        state.push(RunEvent::ToolCallAttempt {
                            tool: tool_call,
                            allowed: false,
                            reason: Some(reason.clone()),
                        });
                        state.messages.push(Message::tool(
                            &tc_req.id,
                            format!("Permission denied: {reason}"),
                        ));
                    } else {
                        // Attribute this turn's LLM cost evenly across capped tools
                        // that are actually executing (not already over-cap).
                        *state.per_tool_spent.entry(key).or_insert(0) +=
                            turn_cost / executing_capped_count;
                        self.execute_allowed_tool(state, &tc_req.id, tool_call)
                            .await;
                    }
                }
                InterceptResult::Denied { reason } => {
                    state.push(RunEvent::ToolCallAttempt {
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
        state.push(RunEvent::ToolCallAttempt {
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
                state.push(RunEvent::ToolCallResult {
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
                state.push(RunEvent::ToolCallResult {
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
    fn finish(mut state: RunState, response: String) -> RunResult {
        state.push(RunEvent::RunComplete {
            total_cost_cents: state.total_cost_cents,
            total_tokens: state.total_tokens,
        });
        let trace = RunTrace::from_events_timed(state.events, state.event_timestamps_ms);
        RunResult {
            response,
            trace,
            total_tokens: state.total_tokens,
            total_cost_cents: state.total_cost_cents,
        }
    }

    /// Export a partial trace to the configured OTEL sink (e.g. on timeout).
    ///
    /// Constructs a synthetic `RunResult` from the partial trace so that the
    /// `StageTimeout` span is reachable even when the run did not complete.
    fn export_partial(
        &self,
        trace: &RunTrace,
        total_tokens: u64,
        total_cost_cents: u64,
        duration: std::time::Duration,
    ) {
        let result = RunResult {
            response: String::new(),
            trace: trace.clone(),
            total_tokens,
            total_cost_cents,
        };
        // #430: mark the OTEL export as partial so dashboards can distinguish
        // timed-out runs from normally-empty completions via rein.run.partial.
        self.apply_otel_export_with_flags(&result, duration, true);
    }

    /// Apply OTEL export after a run completes (side-effect only, never fails loudly).
    fn apply_otel_export(&self, result: &RunResult, duration: std::time::Duration) {
        self.apply_otel_export_with_flags(result, duration, false);
    }

    /// Apply OTEL export with an optional `is_partial` flag.
    ///
    /// When `is_partial` is `true` (e.g. a timed-out run), the root OTEL span
    /// will carry `rein.run.partial = "true"` so dashboards can distinguish
    /// incomplete runs from normally-empty completions.
    fn apply_otel_export_with_flags(
        &self,
        result: &RunResult,
        duration: std::time::Duration,
        is_partial: bool,
    ) {
        let name = self.agent_name.as_deref().unwrap_or("agent");
        match &self.otel_mode {
            OtelMode::None => {}
            OtelMode::FileOnComplete => {
                Self::export_otel_to_file(result, duration, name, is_partial);
            }
            OtelMode::StdoutOnComplete { metrics } => {
                Self::export_otel_to_stdout(result, duration, metrics, name, is_partial);
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
        is_partial: bool,
    ) -> (super::StructuredTrace, chrono::DateTime<chrono::Utc>) {
        let now = chrono::Utc::now();
        let started =
            now - chrono::Duration::from_std(duration).unwrap_or(chrono::Duration::zero());
        let mut trace = result.trace.to_structured(
            agent_name,
            &started.to_rfc3339(),
            &now.to_rfc3339(),
            duration.as_millis().try_into().unwrap_or(u64::MAX),
        );
        trace.is_partial = is_partial;
        (trace, now)
    }

    /// Write OTLP JSON to a timestamped file.
    fn export_otel_to_file(
        result: &RunResult,
        duration: std::time::Duration,
        agent_name: &str,
        is_partial: bool,
    ) {
        let (structured, completed_at) =
            Self::build_structured_trace(result, duration, agent_name, is_partial);
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
        is_partial: bool,
    ) {
        use super::otel_export::to_otlp;
        let (mut structured, _completed_at) =
            Self::build_structured_trace(result, duration, agent_name, is_partial);
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
