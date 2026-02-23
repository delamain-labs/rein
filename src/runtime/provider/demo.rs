//! Mock provider for `rein run --demo`. Returns canned responses that
//! trigger enforcement features (guardrails, circuit breakers, etc.).

use super::{ChatResponse, Message, Provider, ProviderError, ToolDef, Usage};
use std::sync::atomic::{AtomicUsize, Ordering};

/// A demo provider that cycles through canned responses designed to
/// showcase runtime enforcement features.
pub struct DemoProvider {
    call_count: AtomicUsize,
}

impl Default for DemoProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DemoProvider {
    pub fn new() -> Self {
        Self {
            call_count: AtomicUsize::new(0),
        }
    }
}

#[async_trait::async_trait]
impl Provider for DemoProvider {
    async fn chat(
        &self,
        _messages: &[Message],
        _tools: &[ToolDef],
    ) -> Result<ChatResponse, ProviderError> {
        let n = self.call_count.fetch_add(1, Ordering::SeqCst);

        match n % 4 {
            0 => Ok(ChatResponse {
                content: "Sure, here's the customer info you requested: \
                          Name: John Smith, SSN: 123-45-6789, \
                          Credit Card: 4111-1111-1111-1111, \
                          Email: john.smith@example.com. \
                          Let me know if you need anything else!"
                    .to_string(),
                tool_calls: Vec::new(),
                usage: Usage {
                    input_tokens: 50,
                    output_tokens: 80,
                },
                model: "demo-mock".to_string(),
            }),
            1 => Ok(ChatResponse {
                content: "Hello! I'm running through Rein's enforcement \
                          layer. That last response was checked against \
                          your policy's guardrails in real time."
                    .to_string(),
                tool_calls: Vec::new(),
                usage: Usage {
                    input_tokens: 40,
                    output_tokens: 60,
                },
                model: "demo-mock".to_string(),
            }),
            2 => Err(ProviderError::Network(
                "simulated API timeout (demo mode)".to_string(),
            )),
            3 => Ok(ChatResponse {
                content: "Recovery successful. The circuit breaker \
                          transitioned back to half-open after that \
                          simulated failure."
                    .to_string(),
                tool_calls: Vec::new(),
                usage: Usage {
                    input_tokens: 30,
                    output_tokens: 50,
                },
                model: "demo-mock".to_string(),
            }),
            _ => unreachable!(),
        }
    }

    fn name(&self) -> &'static str {
        "demo"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn demo_provider_cycles_responses() {
        let provider = DemoProvider::new();
        let msgs = vec![Message::user("test")];
        let tools = vec![];

        // First: PII response (triggers guardrails)
        let r = provider.chat(&msgs, &tools).await.unwrap();
        assert!(r.content.contains("SSN"));

        // Second: clean response
        let r = provider.chat(&msgs, &tools).await.unwrap();
        assert!(r.content.contains("enforcement"));

        // Third: error (triggers circuit breaker)
        let r = provider.chat(&msgs, &tools).await;
        assert!(r.is_err());

        // Fourth: recovery
        let r = provider.chat(&msgs, &tools).await.unwrap();
        assert!(r.content.contains("Recovery"));

        // Cycles back to PII
        let r = provider.chat(&msgs, &tools).await.unwrap();
        assert!(r.content.contains("SSN"));
    }
}
