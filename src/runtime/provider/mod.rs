use serde::{Deserialize, Serialize};
use std::fmt;

pub mod anthropic;
pub mod openai;
pub mod resolver;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A role in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    /// When role == Tool, the id of the tool call this is responding to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    #[must_use]
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            tool_call_id: None,
        }
    }

    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            tool_call_id: None,
        }
    }

    #[must_use]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            tool_call_id: None,
        }
    }

    #[must_use]
    pub fn tool(call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            tool_call_id: Some(call_id.into()),
        }
    }
}

/// A tool definition exposed to the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    /// JSON Schema for the tool's parameters.
    pub parameters: serde_json::Value,
}

/// A tool call requested by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Token usage from a single LLM call.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// The response from a chat completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Text content of the assistant reply (may be empty if only tool calls).
    pub content: String,
    /// Tool calls the model wants to make.
    pub tool_calls: Vec<ToolCallRequest>,
    /// Token usage.
    pub usage: Usage,
    /// The model string returned by the provider.
    pub model: String,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur when calling a provider.
#[derive(Debug)]
pub enum ProviderError {
    /// HTTP or network error.
    Network(String),
    /// The provider returned a non-success status.
    Api { status: u16, body: String },
    /// Could not parse the provider response.
    Parse(String),
    /// Authentication failed (bad or missing key).
    Auth(String),
    /// Rate limited.
    RateLimited,
}

impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(msg) => write!(f, "network error: {msg}"),
            Self::Api { status, body } => write!(f, "API error ({status}): {body}"),
            Self::Parse(msg) => write!(f, "parse error: {msg}"),
            Self::Auth(msg) => write!(f, "auth error: {msg}"),
            Self::RateLimited => write!(f, "rate limited"),
        }
    }
}

impl std::error::Error for ProviderError {}

// ---------------------------------------------------------------------------
// Provider trait
// ---------------------------------------------------------------------------

/// An LLM provider that can complete chat conversations.
#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    /// Send a chat completion request.
    async fn chat(
        &self,
        messages: &[Message],
        tools: &[ToolDef],
    ) -> Result<ChatResponse, ProviderError>;

    /// Return the provider name (e.g. "openai", "anthropic").
    fn name(&self) -> &'static str;
}

// ---------------------------------------------------------------------------
// Mock provider (for testing)
// ---------------------------------------------------------------------------

/// A mock provider that returns pre-configured responses.
#[derive(Debug, Default)]
pub struct MockProvider {
    responses: std::sync::Mutex<Vec<Result<ChatResponse, String>>>,
}

impl MockProvider {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue a successful response.
    pub fn push_response(&self, response: ChatResponse) {
        self.responses
            .lock()
            .expect("lock poisoned")
            .push(Ok(response));
    }

    /// Queue an error response.
    pub fn push_error(&self, msg: impl Into<String>) {
        self.responses
            .lock()
            .expect("lock poisoned")
            .push(Err(msg.into()));
    }
}

#[async_trait::async_trait]
impl Provider for MockProvider {
    async fn chat(
        &self,
        _messages: &[Message],
        _tools: &[ToolDef],
    ) -> Result<ChatResponse, ProviderError> {
        let mut queue = self.responses.lock().expect("lock poisoned");
        if queue.is_empty() {
            return Err(ProviderError::Api {
                status: 500,
                body: "no mock responses queued".to_string(),
            });
        }
        match queue.remove(0) {
            Ok(resp) => Ok(resp),
            Err(msg) => Err(ProviderError::Network(msg)),
        }
    }

    fn name(&self) -> &'static str {
        "mock"
    }
}
