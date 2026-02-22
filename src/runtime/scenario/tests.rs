use super::*;
use crate::ast::{ScenarioDef, Span};
use std::collections::HashMap;

fn span() -> Span {
    Span { start: 0, end: 0 }
}

fn make_def() -> ScenarioDef {
    ScenarioDef {
        name: "happy_path".to_string(),
        given: vec![
            ("input".to_string(), "refund request".to_string()),
            ("customer_tier".to_string(), "gold".to_string()),
        ],
        expect: vec![
            ("action".to_string(), "approve_refund".to_string()),
            ("response_tone".to_string(), "empathetic".to_string()),
        ],
        span: span(),
    }
}

#[test]
fn passes_when_all_match() {
    let runner = ScenarioRunner::from_def(&make_def());
    let mut actuals = HashMap::new();
    actuals.insert("action".to_string(), "approve_refund".to_string());
    actuals.insert("response_tone".to_string(), "empathetic".to_string());
    assert_eq!(runner.evaluate(&actuals), ScenarioResult::Passed);
}

#[test]
fn fails_on_mismatch() {
    let runner = ScenarioRunner::from_def(&make_def());
    let mut actuals = HashMap::new();
    actuals.insert("action".to_string(), "deny_refund".to_string());
    actuals.insert("response_tone".to_string(), "empathetic".to_string());

    match runner.evaluate(&actuals) {
        ScenarioResult::Failed { failures } => {
            assert_eq!(failures.len(), 1);
            assert_eq!(failures[0].key, "action");
            assert_eq!(failures[0].expected, "approve_refund");
            assert_eq!(failures[0].actual, Some("deny_refund".to_string()));
        }
        ScenarioResult::Passed => panic!("should have failed"),
    }
}

#[test]
fn fails_on_missing_key() {
    let runner = ScenarioRunner::from_def(&make_def());
    let mut actuals = HashMap::new();
    actuals.insert("action".to_string(), "approve_refund".to_string());
    // Missing "response_tone"

    match runner.evaluate(&actuals) {
        ScenarioResult::Failed { failures } => {
            assert_eq!(failures.len(), 1);
            assert_eq!(failures[0].key, "response_tone");
            assert!(failures[0].actual.is_none());
        }
        ScenarioResult::Passed => panic!("should have failed"),
    }
}

#[test]
fn context_returns_given() {
    let runner = ScenarioRunner::from_def(&make_def());
    let ctx = runner.context();
    assert_eq!(ctx["input"], "refund request");
    assert_eq!(ctx["customer_tier"], "gold");
}

#[test]
fn name_returns_scenario_name() {
    let runner = ScenarioRunner::from_def(&make_def());
    assert_eq!(runner.name(), "happy_path");
}

#[test]
fn empty_expectations_always_pass() {
    let def = ScenarioDef {
        name: "empty".to_string(),
        given: vec![],
        expect: vec![],
        span: span(),
    };
    let runner = ScenarioRunner::from_def(&def);
    assert_eq!(runner.evaluate(&HashMap::new()), ScenarioResult::Passed);
}
