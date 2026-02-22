//! Alerting system for workflow anomalies and threshold violations.

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

/// An alert rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub name: String,
    pub condition: AlertCondition,
    pub severity: Severity,
    pub channel: String,
}

/// Alert condition types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    /// Metric exceeds threshold.
    ThresholdExceeded { metric: String, threshold: f64 },
    /// Error rate exceeds percentage.
    ErrorRate { threshold_pct: f64 },
    /// No events received within duration.
    Silence { duration_secs: u64 },
}

/// Alert severity levels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

/// A fired alert.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiredAlert {
    pub rule_name: String,
    pub severity: Severity,
    pub message: String,
    pub timestamp_ms: u64,
}

/// Alert evaluator.
#[derive(Debug, Default)]
pub struct AlertEngine {
    rules: Vec<AlertRule>,
    fired: Vec<FiredAlert>,
}

impl AlertEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an alert rule.
    pub fn add_rule(&mut self, rule: AlertRule) {
        self.rules.push(rule);
    }

    /// Evaluate a metric value against threshold rules.
    pub fn evaluate_metric(&mut self, metric: &str, value: f64) {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX);

        for rule in &self.rules {
            if let AlertCondition::ThresholdExceeded {
                metric: rule_metric,
                threshold,
            } = &rule.condition
                && rule_metric == metric
                && value > *threshold
            {
                self.fired.push(FiredAlert {
                    rule_name: rule.name.clone(),
                    severity: rule.severity.clone(),
                    message: format!("{metric} = {value} exceeds threshold {threshold}"),
                    timestamp_ms: ts,
                });
            }
        }
    }

    /// Get all fired alerts.
    pub fn fired_alerts(&self) -> &[FiredAlert] {
        &self.fired
    }

    /// Clear fired alerts.
    pub fn clear(&mut self) {
        self.fired.clear();
    }
}
