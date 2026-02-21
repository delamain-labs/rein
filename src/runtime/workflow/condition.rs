//! Condition matching for workflow routing.
//!
//! Supports multiple matching strategies: exact equality, substring containment,
//! regular expressions, and JSON path extraction.

use crate::ast::ConditionMatcher;

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
            regex::Regex::new(pattern)
                .map(|re| re.is_match(val))
                .unwrap_or(false)
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
