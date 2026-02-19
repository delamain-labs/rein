use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{
    ChatResponse, Message, Provider, ProviderError, Role, ToolCallRequest, ToolDef, Usage,
};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// OpenAI-specific request/response types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<OaiMessage<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<OaiTool<'a>>,
}

#[derive(Serialize)]
struct OaiMessage<'a> {
    role: &'a str,
    content: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<&'a str>,
}

#[derive(Serialize)]
struct OaiTool<'a> {
    r#type: &'a str,
    function: OaiFunction<'a>,
}

#[derive(Serialize)]
struct OaiFunction<'a> {
    name: &'a str,
    description: &'a str,
    parameters: &'a serde_json::Value,
}

#[derive(Deserialize)]
struct OaiResponse {
    choices: Vec<OaiChoice>,
    usage: Option<OaiUsage>,
    model: String,
}

#[derive(Deserialize)]
struct OaiChoice {
    message: OaiAssistantMessage,
}

#[derive(Deserialize)]
struct OaiAssistantMessage {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<OaiToolCall>,
}

#[derive(Deserialize)]
struct OaiToolCall {
    id: String,
    function: OaiFunctionCall,
}

#[derive(Deserialize)]
struct OaiFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct OaiUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
}

#[derive(Deserialize)]
struct OaiErrorResponse {
    error: OaiErrorDetail,
}

#[derive(Deserialize)]
struct OaiErrorDetail {
    message: String,
}

// ---------------------------------------------------------------------------
// OpenAI Provider
// ---------------------------------------------------------------------------

/// An LLM provider backed by the `OpenAI` Chat Completions API.
pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAiProvider {
    /// Create a new `OpenAiProvider`.
    ///
    /// # Arguments
    /// * `api_key` - `OpenAI` API key
    /// * `model` - Model identifier (e.g. `gpt-4o`)
    /// * `base_url` - Optional base URL override (for proxies / compatible APIs)
    #[must_use]
    pub fn new(
        api_key: impl Into<String>,
        model: impl Into<String>,
        base_url: Option<String>,
    ) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            model: model.into(),
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        }
    }

    fn convert_role(role: Role) -> &'static str {
        match role {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
        }
    }
}

#[async_trait::async_trait]
impl Provider for OpenAiProvider {
    async fn chat(
        &self,
        messages: &[Message],
        tools: &[ToolDef],
    ) -> Result<ChatResponse, ProviderError> {
        let oai_messages: Vec<OaiMessage<'_>> = messages
            .iter()
            .map(|m| OaiMessage {
                role: Self::convert_role(m.role),
                content: &m.content,
                tool_call_id: m.tool_call_id.as_deref(),
            })
            .collect();

        let oai_tools: Vec<OaiTool<'_>> = tools
            .iter()
            .map(|t| OaiTool {
                r#type: "function",
                function: OaiFunction {
                    name: &t.name,
                    description: &t.description,
                    parameters: &t.parameters,
                },
            })
            .collect();

        let body = ChatRequest {
            model: &self.model,
            messages: oai_messages,
            tools: oai_tools,
        };

        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;

        let status = response.status().as_u16();

        if status == 401 {
            let text = response.text().await.unwrap_or_default();
            return Err(ProviderError::Auth(text));
        }

        if status == 429 {
            return Err(ProviderError::RateLimited);
        }

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            let body_msg = serde_json::from_str::<OaiErrorResponse>(&text)
                .map(|e| e.error.message)
                .unwrap_or(text);
            return Err(ProviderError::Api {
                status,
                body: body_msg,
            });
        }

        let oai: OaiResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        let choice = oai
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| ProviderError::Parse("no choices in response".to_string()))?;

        let tool_calls: Vec<ToolCallRequest> = choice
            .message
            .tool_calls
            .into_iter()
            .map(|tc| {
                let arguments = serde_json::from_str(&tc.function.arguments)
                    .unwrap_or(serde_json::Value::String(tc.function.arguments));
                ToolCallRequest {
                    id: tc.id,
                    name: tc.function.name,
                    arguments,
                }
            })
            .collect();

        let usage = oai.usage.map_or(Usage::default(), |u| Usage {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
        });

        Ok(ChatResponse {
            content: choice.message.content.unwrap_or_default(),
            tool_calls,
            usage,
            model: oai.model,
        })
    }

    fn name(&self) -> &'static str {
        "openai"
    }
}
