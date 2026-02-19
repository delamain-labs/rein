use super::ToolCall;

#[cfg(test)]
mod tests;

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
    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolOutput, ExecutorError>;
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
    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolOutput, ExecutorError> {
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
