//! Event streaming for workflow and agent lifecycle events.

use serde::{Deserialize, Serialize};
use std::sync::mpsc;

#[cfg(test)]
mod tests;

/// A runtime event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub kind: EventKind,
    pub source: String,
    pub data: serde_json::Value,
    pub timestamp_ms: u64,
}

/// Event categories.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    WorkflowStarted,
    WorkflowCompleted,
    WorkflowFailed,
    StepStarted,
    StepCompleted,
    StepFailed,
    AgentInvoked,
    ToolCalled,
    GuardrailTriggered,
    EscalationRaised,
}

/// Simple in-process event bus.
pub struct EventBus {
    sender: mpsc::Sender<Event>,
    receiver: mpsc::Receiver<Event>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self { sender, receiver }
    }

    /// Get a sender handle that can be cloned for multiple producers.
    pub fn sender(&self) -> mpsc::Sender<Event> {
        self.sender.clone()
    }

    /// Emit an event.
    pub fn emit(&self, event: Event) -> Result<(), mpsc::SendError<Event>> {
        self.sender.send(event)
    }

    /// Drain all pending events.
    pub fn drain(&self) -> Vec<Event> {
        let mut events = Vec::new();
        while let Ok(event) = self.receiver.try_recv() {
            events.push(event);
        }
        events
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to create an event with current timestamp.
pub fn make_event(kind: EventKind, source: impl Into<String>, data: serde_json::Value) -> Event {
    use std::time::{SystemTime, UNIX_EPOCH};
    Event {
        kind,
        source: source.into(),
        data,
        timestamp_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX),
    }
}
