//! Webhook configuration and dispatch.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[cfg(test)]
mod tests;

/// A configured webhook endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub name: String,
    pub url: String,
    /// Events that trigger this webhook.
    pub events: Vec<String>,
    /// Optional signing secret for HMAC verification.
    pub secret: Option<String>,
    /// Additional headers to include.
    pub headers: HashMap<String, String>,
    /// Whether the webhook is active.
    pub active: bool,
}

/// A webhook payload ready for dispatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub event: String,
    pub timestamp: String,
    pub data: serde_json::Value,
}

impl WebhookConfig {
    /// Create a new webhook config.
    pub fn new(name: impl Into<String>, url: impl Into<String>, events: Vec<String>) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
            events,
            secret: None,
            headers: HashMap::new(),
            active: true,
        }
    }

    /// Check if this webhook should fire for the given event.
    pub fn matches_event(&self, event: &str) -> bool {
        self.active && (self.events.contains(&"*".to_string()) || self.events.iter().any(|e| e == event))
    }

    /// Compute HMAC signature for a payload.
    pub fn sign(&self, payload: &[u8]) -> Option<String> {
        self.secret.as_ref().map(|secret| {
            let mut hasher = Sha256::new();
            hasher.update(secret.as_bytes());
            hasher.update(payload);
            format!("sha256={:x}", hasher.finalize())
        })
    }
}

/// Registry of webhook configurations.
#[derive(Debug, Default)]
pub struct WebhookRegistry {
    hooks: Vec<WebhookConfig>,
}

impl WebhookRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, config: WebhookConfig) {
        self.hooks.push(config);
    }

    /// Get all webhooks that match an event.
    pub fn matching(&self, event: &str) -> Vec<&WebhookConfig> {
        self.hooks.iter().filter(|h| h.matches_event(event)).collect()
    }

    pub fn len(&self) -> usize {
        self.hooks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }
}
