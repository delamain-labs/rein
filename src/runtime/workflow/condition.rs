//! Condition matching for workflow routing.
//!
//! Supports multiple matching strategies: exact equality, substring containment,
//! regular expressions, and JSON path extraction.

use std::collections::HashMap;

use tracing::warn;

use crate::ast::{CompareOp, ConditionMatcher, WhenExpr, WhenValue};

/// Extract the value of a `field: value` or `field=value` line from output.
fn extract_field_value<'a>(output: &'a str, field: &str) -> Option<&'a str> {
    let field_lower = field.to_lowercase();
    for line in output.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();
        if let Some(rest) = lower.strip_prefix(&field_lower) {
            let rest = rest.trim_start();
            if rest.starts_with(':') || rest.starts_with('=') {
                // Return from original (non-lowered) trimmed line
                let offset = trimmed.len() - rest.len() + 1; // skip ':' or '='
                return Some(trimmed[offset..].trim());
            }
        }
    }
    None
}

/// Check whether a conditional route matches the agent output.
///
/// Supports multiple matching strategies via [`ConditionMatcher`].
pub fn condition_matches(output: &str, field: &str, matcher: &ConditionMatcher) -> bool {
    match matcher {
        ConditionMatcher::Equals(expected) => {
            let Some(val) = extract_field_value(output, field) else {
                return false;
            };
            let val_lower = val.to_lowercase();
            let expected_lower = expected.to_lowercase();
            val_lower == expected_lower
                || val_lower.strip_prefix(&expected_lower).is_some_and(|rest| {
                    rest.starts_with(|c: char| !c.is_alphanumeric() && c != '_')
                })
        }
        ConditionMatcher::Contains(needle) => {
            let Some(val) = extract_field_value(output, field) else {
                return false;
            };
            val.to_lowercase().contains(&needle.to_lowercase())
        }
        ConditionMatcher::Regex(pattern) => {
            let Some(val) = extract_field_value(output, field) else {
                return false;
            };
            match regex::Regex::new(pattern) {
                Ok(re) => re.is_match(val),
                Err(err) => {
                    warn!("invalid regex pattern '{pattern}': {err}");
                    false
                }
            }
        }
        ConditionMatcher::JsonPath { path, expected } => json_path_matches(output, path, expected),
    }
}

/// Match a JSON path expression against output parsed as JSON.
fn json_path_matches(output: &str, path: &str, expected: &str) -> bool {
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(output) else {
        return false;
    };
    let value = resolve_json_path(&parsed, path);
    match value {
        Some(serde_json::Value::String(s)) => s.eq_ignore_ascii_case(expected),
        Some(serde_json::Value::Number(n)) => n.to_string() == expected,
        Some(serde_json::Value::Bool(b)) => b.to_string() == expected,
        _ => false,
    }
}

/// Resolve a dot-separated JSON path (e.g. `result.status`) against a value.
fn resolve_json_path<'a>(
    value: &'a serde_json::Value,
    path: &str,
) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

/// Evaluate a `when:` expression against the current set of step outputs.
///
/// Returns `true` if the guard condition is satisfied (step should run),
/// `false` if the condition is not met (step should be skipped).
///
/// Each value in `outputs` is the raw text output produced by the named step.
/// Numeric comparisons parse the extracted field value as `f64`; non-parseable
/// values are treated as 0.
pub fn when_expr_matches(expr: &WhenExpr, outputs: &HashMap<String, String>) -> bool {
    match expr {
        WhenExpr::And(exprs) => exprs.iter().all(|e| when_expr_matches(e, outputs)),
        WhenExpr::Or(exprs) => exprs.iter().any(|e| when_expr_matches(e, outputs)),
        WhenExpr::Comparison(cmp) => {
            // Search all prior step outputs for the field value.
            let raw_value = outputs
                .values()
                .find_map(|output| extract_field_value(output, &cmp.field));

            let Some(raw) = raw_value else {
                // Field not found — condition cannot be satisfied.
                return false;
            };

            match &cmp.value {
                WhenValue::String(expected) | WhenValue::Ident(expected) => {
                    raw.to_lowercase() == expected.to_lowercase()
                }
                WhenValue::Number(rhs) | WhenValue::Percent(rhs) => {
                    let lhs: f64 = raw
                        .trim_end_matches('%')
                        .parse()
                        .unwrap_or(0.0);
                    let rhs_val: f64 = rhs
                        .trim_end_matches('%')
                        .parse()
                        .unwrap_or(0.0);
                    compare_numeric(lhs, &cmp.op, rhs_val)
                }
                WhenValue::Currency { amount, .. } => {
                    let lhs: f64 = raw
                        .trim_start_matches(|c: char| !c.is_ascii_digit() && c != '.')
                        .parse()
                        .unwrap_or(0.0);
                    #[allow(clippy::cast_precision_loss)]
                    let rhs = *amount as f64;
                    compare_numeric(lhs, &cmp.op, rhs)
                }
            }
        }
    }
}

fn compare_numeric(lhs: f64, op: &CompareOp, rhs: f64) -> bool {
    match op {
        CompareOp::Lt => lhs < rhs,
        CompareOp::Gt => lhs > rhs,
        CompareOp::LtEq => lhs <= rhs,
        CompareOp::GtEq => lhs >= rhs,
        CompareOp::Eq => (lhs - rhs).abs() < f64::EPSILON,
        CompareOp::NotEq => (lhs - rhs).abs() >= f64::EPSILON,
    }
}
