//! Eval block runtime: runs assertions against datasets.

use crate::ast::{CompareOp, EvalAssertion, EvalDef};

#[cfg(test)]
mod tests;

/// Result of evaluating a single assertion.
#[derive(Debug, Clone)]
pub struct AssertionResult {
    pub metric: String,
    pub expected: String,
    pub actual: f64,
    pub passed: bool,
}

/// Result of evaluating an entire eval block.
#[derive(Debug, Clone)]
pub struct EvalResult {
    pub name: Option<String>,
    pub dataset: String,
    pub assertions: Vec<AssertionResult>,
    pub passed: bool,
}

/// Evaluate a single assertion against a metric value.
#[must_use]
pub fn check_assertion(assertion: &EvalAssertion, actual: f64) -> AssertionResult {
    let threshold = parse_threshold(&assertion.value);
    let passed = match assertion.op {
        CompareOp::Lt => actual < threshold,
        CompareOp::Gt => actual > threshold,
        CompareOp::LtEq => actual <= threshold,
        CompareOp::GtEq => actual >= threshold,
        CompareOp::Eq => (actual - threshold).abs() < f64::EPSILON,
        CompareOp::NotEq => (actual - threshold).abs() >= f64::EPSILON,
    };

    AssertionResult {
        metric: assertion.metric.clone(),
        expected: assertion.value.clone(),
        actual,
        passed,
    }
}

/// Run all assertions in an eval block against provided metrics.
///
/// `metrics` is a function that returns the actual value for a given metric name.
#[must_use]
pub fn run_eval<F>(eval: &EvalDef, metrics: F) -> EvalResult
where
    F: Fn(&str) -> Option<f64>,
{
    let assertions: Vec<AssertionResult> = eval
        .assertions
        .iter()
        .map(|a| {
            let actual = metrics(&a.metric).unwrap_or(0.0);
            check_assertion(a, actual)
        })
        .collect();

    let passed = assertions.iter().all(|a| a.passed);

    EvalResult {
        name: eval.name.clone(),
        dataset: eval.dataset.clone(),
        assertions,
        passed,
    }
}

fn parse_threshold(value: &str) -> f64 {
    // Strip trailing % if present
    let cleaned = value.trim_end_matches('%');
    cleaned.parse::<f64>().unwrap_or(0.0)
}
