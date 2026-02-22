use super::*;
use crate::ast::{CompareOp, EvalAssertion, EvalDef, Span};

fn make_assertion(metric: &str, op: CompareOp, value: &str) -> EvalAssertion {
    EvalAssertion {
        metric: metric.to_string(),
        op,
        value: value.to_string(),
        span: Span::new(0, 0),
    }
}

#[test]
fn check_gt_passes() {
    let a = make_assertion("accuracy", CompareOp::Gt, "90");
    let result = check_assertion(&a, 95.0);
    assert!(result.passed);
}

#[test]
fn check_gt_fails() {
    let a = make_assertion("accuracy", CompareOp::Gt, "90");
    let result = check_assertion(&a, 85.0);
    assert!(!result.passed);
}

#[test]
fn check_lt_passes() {
    let a = make_assertion("latency", CompareOp::Lt, "2000");
    let result = check_assertion(&a, 1500.0);
    assert!(result.passed);
}

#[test]
fn check_gte_passes() {
    let a = make_assertion("accuracy", CompareOp::GtEq, "90");
    let result = check_assertion(&a, 90.0);
    assert!(result.passed);
}

#[test]
fn check_eq_passes() {
    let a = make_assertion("count", CompareOp::Eq, "42");
    let result = check_assertion(&a, 42.0);
    assert!(result.passed);
}

#[test]
fn check_neq_passes() {
    let a = make_assertion("errors", CompareOp::NotEq, "0");
    let result = check_assertion(&a, 3.0);
    assert!(result.passed);
}

#[test]
fn check_percentage_threshold() {
    let a = make_assertion("accuracy", CompareOp::Gt, "90%");
    let result = check_assertion(&a, 95.0);
    assert!(result.passed);
}

#[test]
fn run_eval_all_pass() {
    let eval = EvalDef {
        name: Some("quality".to_string()),
        dataset: "test.jsonl".to_string(),
        assertions: vec![
            make_assertion("accuracy", CompareOp::Gt, "90"),
            make_assertion("latency", CompareOp::Lt, "2000"),
        ],
        on_failure: None,
        span: Span::new(0, 0),
    };

    let result = run_eval(&eval, |metric| match metric {
        "accuracy" => Some(95.0),
        "latency" => Some(1500.0),
        _ => None,
    });

    assert!(result.passed);
    assert_eq!(result.assertions.len(), 2);
}

#[test]
fn run_eval_one_fails() {
    let eval = EvalDef {
        name: Some("quality".to_string()),
        dataset: "test.jsonl".to_string(),
        assertions: vec![
            make_assertion("accuracy", CompareOp::Gt, "90"),
            make_assertion("latency", CompareOp::Lt, "1000"),
        ],
        on_failure: None,
        span: Span::new(0, 0),
    };

    let result = run_eval(&eval, |metric| match metric {
        "accuracy" => Some(95.0),
        "latency" => Some(1500.0),
        _ => None,
    });

    assert!(!result.passed);
    assert!(result.assertions[0].passed);
    assert!(!result.assertions[1].passed);
}

#[test]
fn run_eval_missing_metric_defaults_to_zero() {
    let eval = EvalDef {
        name: None,
        dataset: "test.jsonl".to_string(),
        assertions: vec![make_assertion("missing", CompareOp::Gt, "0")],
        on_failure: None,
        span: Span::new(0, 0),
    };

    let result = run_eval(&eval, |_| None);
    assert!(!result.passed); // 0.0 is not > 0.0
}
