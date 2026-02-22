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
        EventKind::StepStarted,
        "step1",
        serde_json::json!({}),
    ))
    .unwrap();
    let events = bus.drain();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].kind, EventKind::WorkflowStarted);
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
