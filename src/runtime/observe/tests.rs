use super::*;
use crate::ast::{ObserveDef, Span};

fn span() -> Span {
    Span { start: 0, end: 0 }
}

fn make_def() -> ObserveDef {
    ObserveDef {
        name: "agent_monitor".to_string(),
        trace: Some("all steps".to_string()),
        metrics: vec!["latency".to_string(), "accuracy".to_string()],
        alert_when: None,
        export: Some("prometheus".to_string()),
        span: span(),
    }
}

#[test]
fn creates_from_def() {
    let def = make_def();
    let obs = Observer::from_def(&def);
    assert_eq!(obs.name(), "agent_monitor");
    assert_eq!(obs.tracked_metrics().len(), 2);
    assert_eq!(obs.export_target(), Some("prometheus"));
}

#[test]
fn records_and_retrieves_metrics() {
    let def = make_def();
    let mut obs = Observer::from_def(&def);
    obs.record("latency", 150.0);
    obs.record("accuracy", 0.95);
    assert_eq!(obs.metrics().len(), 2);
    assert_eq!(obs.latest("latency"), Some(150.0));
    assert_eq!(obs.latest("accuracy"), Some(0.95));
}

#[test]
fn latest_returns_most_recent() {
    let def = make_def();
    let mut obs = Observer::from_def(&def);
    obs.record("latency", 100.0);
    obs.record("latency", 200.0);
    obs.record("latency", 50.0);
    assert_eq!(obs.latest("latency"), Some(50.0));
}

#[test]
fn latest_returns_none_for_unknown() {
    let def = make_def();
    let obs = Observer::from_def(&def);
    assert_eq!(obs.latest("unknown"), None);
}

#[test]
fn clear_removes_all() {
    let def = make_def();
    let mut obs = Observer::from_def(&def);
    obs.record("latency", 100.0);
    obs.clear();
    assert!(obs.metrics().is_empty());
}
