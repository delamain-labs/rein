use super::*;

#[test]
fn emit_and_drain() {
    let bus = EventBus::new();
    bus.emit(make_event(
        EventKind::WorkflowStarted,
        "wf1",
        serde_json::json!({"trigger": "new"}),
    ))
    .unwrap();
    bus.emit(make_event(
        EventKind::AgentInvoked,
        "agent1",
        serde_json::json!({}),
    ))
    .unwrap();
    let events = bus.drain();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].kind, EventKind::WorkflowStarted);
}

/// #381: EventKind must not contain step-lifecycle variants that duplicate
/// RunEvent (the canonical step-lifecycle event system). This exhaustive match
/// fails to compile if any of the removed variants (StepStarted, StepCompleted,
/// StepFailed, StepSkipped) are re-introduced.
#[test]
fn event_kind_has_no_step_lifecycle_duplicates() {
    let variants = [
        EventKind::WorkflowStarted,
        EventKind::WorkflowCompleted,
        EventKind::WorkflowFailed,
        EventKind::AgentInvoked,
        EventKind::ToolCalled,
        EventKind::GuardrailTriggered,
        EventKind::EscalationRaised,
    ];
    // Exhaustive match — any re-added step-lifecycle variant would be an
    // uncovered arm and produce a compile error here.
    for kind in variants {
        let _ = match kind {
            EventKind::WorkflowStarted => "wf_started",
            EventKind::WorkflowCompleted => "wf_completed",
            EventKind::WorkflowFailed => "wf_failed",
            EventKind::AgentInvoked => "agent",
            EventKind::ToolCalled => "tool",
            EventKind::GuardrailTriggered => "guardrail",
            EventKind::EscalationRaised => "escalation",
        };
    }
}

#[test]
fn drain_empty() {
    let bus = EventBus::new();
    assert!(bus.drain().is_empty());
}

#[test]
fn sender_clone() {
    let bus = EventBus::new();
    let s = bus.sender();
    s.send(make_event(
        EventKind::ToolCalled,
        "tool",
        serde_json::json!({}),
    ))
    .unwrap();
    assert_eq!(bus.drain().len(), 1);
}
