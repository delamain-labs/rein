use std::collections::HashMap;

use super::ToolCall;

/// A map of resolved secrets keyed by binding name.
///
/// Implements `Debug` with redacted output so that secrets are never
/// accidentally written to logs or OTEL spans.
pub struct Secrets(HashMap<String, String>);

impl std::fmt::Debug for Secrets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Secrets([redacted, {} keys])", self.0.len())
    }
}

impl From<HashMap<String, String>> for Secrets {
    fn from(map: HashMap<String, String>) -> Self {
        Self(map)
    }
}

impl Secrets {
    /// Look up a secret by binding name.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&String> {
        self.0.get(key)
    }

    /// Returns `true` if there are no secrets.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Number of secrets.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[cfg(test)]
mod tests;

/// Context passed to every tool execution, carrying both the call parameters
/// and the resolved secrets injected by the enclosing `secrets { }` block.
/// Secrets are never emitted to traces or OTEL spans.
pub struct ToolCallContext<'a> {
    /// The tool call to execute.
    pub tool_call: &'a ToolCall,
    /// Resolved secrets keyed by binding name. Debug output is redacted.
    pub secrets: &'a Secrets,
}

/// The result of executing a tool.
#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub success: bool,
    pub output: String,
}

/// Errors from tool execution.
#[derive(Debug)]
pub enum ExecutorError {
    /// The tool is not registered or available.
    NotFound(String),
    /// The tool execution failed.
    Failed(String),
}

impl std::fmt::Display for ExecutorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(name) => write!(f, "tool not found: {name}"),
            Self::Failed(msg) => write!(f, "tool execution failed: {msg}"),
        }
    }
}

impl std::error::Error for ExecutorError {}

/// Executes tool calls. Implementations can be mock (for testing) or real
/// (for production use with actual APIs).
#[async_trait::async_trait]
pub trait ToolExecutor: Send + Sync {
    /// Execute a tool call and return its output.
    async fn execute(&self, ctx: &ToolCallContext<'_>) -> Result<ToolOutput, ExecutorError>;
}

// ---------------------------------------------------------------------------
// Noop executor (for agents with no tools)
// ---------------------------------------------------------------------------

/// An executor that denies all tool calls. Use for chat-only agents.
pub struct NoopExecutor;

#[async_trait::async_trait]
impl ToolExecutor for NoopExecutor {
    async fn execute(&self, ctx: &ToolCallContext<'_>) -> Result<ToolOutput, ExecutorError> {
        Err(ExecutorError::NotFound(format!(
            "{}.{}",
            ctx.tool_call.namespace, ctx.tool_call.action
        )))
    }
}

// ---------------------------------------------------------------------------
// Mock executor
// ---------------------------------------------------------------------------

/// A mock executor that returns pre-configured responses for specific tools.
#[derive(Debug, Default)]
pub struct MockExecutor {
    handlers: std::sync::Mutex<Vec<MockHandler>>,
}

#[derive(Debug)]
struct MockHandler {
    namespace: String,
    action: String,
    response: Result<ToolOutput, String>,
}

impl MockExecutor {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a successful response for a tool.
    pub fn on_call(&self, namespace: &str, action: &str, output: impl Into<String>) {
        self.handlers
            .lock()
            .expect("lock poisoned")
            .push(MockHandler {
                namespace: namespace.to_string(),
                action: action.to_string(),
                response: Ok(ToolOutput {
                    success: true,
                    output: output.into(),
                }),
            });
    }

    /// Register a failure response for a tool.
    pub fn on_call_fail(&self, namespace: &str, action: &str, error: impl Into<String>) {
        self.handlers
            .lock()
            .expect("lock poisoned")
            .push(MockHandler {
                namespace: namespace.to_string(),
                action: action.to_string(),
                response: Err(error.into()),
            });
    }
}

#[async_trait::async_trait]
impl ToolExecutor for MockExecutor {
    async fn execute(&self, ctx: &ToolCallContext<'_>) -> Result<ToolOutput, ExecutorError> {
        let tool_call = ctx.tool_call;
        let handlers = self.handlers.lock().expect("lock poisoned");
        for handler in handlers.iter() {
            if handler.namespace == tool_call.namespace && handler.action == tool_call.action {
                return match &handler.response {
                    Ok(output) => Ok(output.clone()),
                    Err(msg) => Err(ExecutorError::Failed(msg.clone())),
                };
            }
        }
        Err(ExecutorError::NotFound(format!(
            "{}.{}",
            tool_call.namespace, tool_call.action
        )))
    }
}
