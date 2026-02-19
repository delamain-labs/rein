use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{
    ChatResponse, Message, Provider, ProviderError, Role, ToolCallRequest, ToolDef, Usage,
};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Anthropic-specific request/response types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct AnthropicRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<&'a str>,
    messages: Vec<AnthropicMessage<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<AnthropicTool<'a>>,
}

#[derive(Serialize)]
struct AnthropicMessage<'a> {
    role: &'a str,
    content: AnthropicContent<'a>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum AnthropicContent<'a> {
    Text(&'a str),
    Blocks(Vec<ContentBlock>),
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Serialize)]
struct AnthropicTool<'a> {
    name: &'a str,
    description: &'a str,
    input_schema: &'a serde_json::Value,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<ResponseContent>,
    model: String,
    usage: AnthropicUsage,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ResponseContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u64,
    output_tokens: u64,
}

#[derive(Deserialize)]
struct AnthropicError {
    error: AnthropicErrorDetail,
}

#[derive(Deserialize)]
struct AnthropicErrorDetail {
    message: String,
}

// ---------------------------------------------------------------------------
// Anthropic Provider
// ---------------------------------------------------------------------------

/// An LLM provider backed by the Anthropic Messages API.
pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
    max_tokens: u32,
}

impl AnthropicProvider {
    /// Create a new `AnthropicProvider`.
    #[must_use]
    pub fn new(
        api_key: impl Into<String>,
        model: impl Into<String>,
        base_url: Option<String>,
        max_tokens: Option<u32>,
    ) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            model: model.into(),
            base_url: base_url.unwrap_or_else(|| "https://api.anthropic.com".to_string()),
            max_tokens: max_tokens.unwrap_or(4096),
        }
    }

    /// Build the messages list, extracting system messages separately.
    fn build_messages(messages: &[Message]) -> (Option<&str>, Vec<AnthropicMessage<'_>>) {
        let mut system = None;
        let mut out = Vec::new();

        for msg in messages {
            match msg.role {
                Role::System => {
                    system = Some(msg.content.as_str());
                }
                Role::User => {
                    out.push(AnthropicMessage {
                        role: "user",
                        content: AnthropicContent::Text(&msg.content),
                    });
                }
                Role::Assistant => {
                    out.push(AnthropicMessage {
                        role: "assistant",
                        content: AnthropicContent::Text(&msg.content),
                    });
                }
                Role::Tool => {
                    let tool_call_id = msg.tool_call_id.as_deref().unwrap_or("unknown");
                    out.push(AnthropicMessage {
                        role: "user",
                        content: AnthropicContent::Blocks(vec![ContentBlock::ToolResult {
                            tool_use_id: tool_call_id.to_string(),
                            content: msg.content.clone(),
                        }]),
                    });
                }
            }
        }

        (system, out)
    }
}

#[async_trait::async_trait]
impl Provider for AnthropicProvider {
    async fn chat(
        &self,
        messages: &[Message],
        tools: &[ToolDef],
    ) -> Result<ChatResponse, ProviderError> {
        let (system, anthro_messages) = Self::build_messages(messages);

        let anthro_tools: Vec<AnthropicTool<'_>> = tools
            .iter()
            .map(|t| AnthropicTool {
                name: &t.name,
                description: &t.description,
                input_schema: &t.parameters,
            })
            .collect();

        let body = AnthropicRequest {
            model: &self.model,
            max_tokens: self.max_tokens,
            system,
            messages: anthro_messages,
            tools: anthro_tools,
        };

        let url = format!("{}/v1/messages", self.base_url);

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
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
            let body_msg = serde_json::from_str::<AnthropicError>(&text)
                .map(|e| e.error.message)
                .unwrap_or(text);
            return Err(ProviderError::Api {
                status,
                body: body_msg,
            });
        }

        let anthro: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        let mut text_content = String::new();
        let mut tool_calls = Vec::new();

        for block in anthro.content {
            match block {
                ResponseContent::Text { text } => {
                    if !text_content.is_empty() {
                        text_content.push('\n');
                    }
                    text_content.push_str(&text);
                }
                ResponseContent::ToolUse { id, name, input } => {
                    tool_calls.push(ToolCallRequest {
                        id,
                        name,
                        arguments: input,
                    });
                }
            }
        }

        Ok(ChatResponse {
            content: text_content,
            tool_calls,
            usage: Usage {
                input_tokens: anthro.usage.input_tokens,
                output_tokens: anthro.usage.output_tokens,
            },
            model: anthro.model,
        })
    }

    fn name(&self) -> &'static str {
        "anthropic"
    }
}
