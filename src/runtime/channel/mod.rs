use crate::ast::ChannelDef;
use std::collections::VecDeque;

#[cfg(test)]
mod tests;

/// A message on a channel.
#[derive(Debug, Clone)]
pub struct ChannelMessage {
    pub sender: String,
    pub payload: String,
}

/// A runtime channel for agent-to-agent messaging.
#[derive(Debug)]
pub struct Channel {
    name: String,
    message_type: Option<String>,
    retention: Option<String>,
    messages: VecDeque<ChannelMessage>,
}

impl Channel {
    /// Create from a parsed channel definition.
    #[must_use]
    pub fn from_def(def: &ChannelDef) -> Self {
        Self {
            name: def.name.clone(),
            message_type: def.message_type.clone(),
            retention: def.retention.clone(),
            messages: VecDeque::new(),
        }
    }

    /// Get the channel name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Publish a message to the channel.
    pub fn publish(&mut self, sender: &str, payload: &str) {
        self.messages.push_back(ChannelMessage {
            sender: sender.to_string(),
            payload: payload.to_string(),
        });
    }

    /// Consume the next message, if any.
    pub fn consume(&mut self) -> Option<ChannelMessage> {
        self.messages.pop_front()
    }

    /// Peek at the next message without consuming.
    #[must_use]
    pub fn peek(&self) -> Option<&ChannelMessage> {
        self.messages.front()
    }

    /// Number of messages in the channel.
    #[must_use]
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Whether the channel is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Get the configured message type.
    #[must_use]
    pub fn message_type(&self) -> Option<&str> {
        self.message_type.as_deref()
    }

    /// Get the configured retention policy.
    #[must_use]
    pub fn retention(&self) -> Option<&str> {
        self.retention.as_deref()
    }
}
