//! Observability exports for OTLP, Datadog, and Prometheus.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

#[cfg(test)]
mod tests;

/// A metric data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    pub labels: HashMap<String, String>,
    pub timestamp_ms: u64,
}

/// Export format for metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Prometheus,
    Otlp,
    Datadog,
    Json,
}

/// In-process metrics collector.
pub struct MetricsCollector {
    metrics: Mutex<Vec<Metric>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Mutex::new(Vec::new()),
        }
    }

    /// Record a metric.
    pub fn record(&self, name: impl Into<String>, value: f64, labels: HashMap<String, String>) {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX);
        self.metrics
            .lock()
            .expect("metrics mutex poisoned")
            .push(Metric {
                name: name.into(),
                value,
                labels,
                timestamp_ms: ts,
            });
    }

    /// Export all metrics in the given format.
    pub fn export(&self, format: &ExportFormat) -> String {
        let metrics = self.metrics.lock().expect("metrics mutex poisoned");
        match format {
            ExportFormat::Prometheus => export_prometheus(&metrics),
            ExportFormat::Json => serde_json::to_string_pretty(&*metrics).unwrap_or_default(),
            ExportFormat::Otlp | ExportFormat::Datadog => {
                // Placeholder: real implementations would use protocol-specific formats
                serde_json::to_string(&*metrics).unwrap_or_default()
            }
        }
    }

    /// Number of recorded metrics.
    pub fn len(&self) -> usize {
        self.metrics.lock().expect("metrics mutex poisoned").len()
    }

    /// Whether any metrics have been recorded.
    pub fn is_empty(&self) -> bool {
        self.metrics
            .lock()
            .expect("metrics mutex poisoned")
            .is_empty()
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

fn export_prometheus(metrics: &[Metric]) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    for m in metrics {
        out.push_str(&m.name);
        if !m.labels.is_empty() {
            out.push('{');
            let labels: Vec<String> = m
                .labels
                .iter()
                .map(|(k, v)| format!("{k}=\"{v}\""))
                .collect();
            out.push_str(&labels.join(","));
            out.push('}');
        }
        let _ = writeln!(out, " {}", m.value);
    }
    out
}
