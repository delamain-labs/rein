use crate::ast::ObserveDef;

#[cfg(test)]
mod tests;

/// A collected metric value.
#[derive(Debug, Clone)]
pub struct MetricValue {
    pub name: String,
    pub value: f64,
}

/// An alert triggered by an observe block.
#[derive(Debug, Clone)]
pub struct Alert {
    pub observer_name: String,
    pub message: String,
}

/// The observer collects metrics and checks alert conditions.
#[derive(Debug)]
pub struct Observer {
    name: String,
    metrics: Vec<String>,
    export_target: Option<String>,
    collected: Vec<MetricValue>,
}

impl Observer {
    /// Create from a parsed observe definition.
    #[must_use]
    pub fn from_def(def: &ObserveDef) -> Self {
        Self {
            name: def.name.clone(),
            metrics: def.metrics.clone(),
            export_target: def.export.clone(),
            collected: Vec::new(),
        }
    }

    /// Record a metric value.
    pub fn record(&mut self, name: &str, value: f64) {
        self.collected.push(MetricValue {
            name: name.to_string(),
            value,
        });
    }

    /// Get all collected metrics.
    #[must_use]
    pub fn metrics(&self) -> &[MetricValue] {
        &self.collected
    }

    /// Get the observer name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get tracked metric names.
    #[must_use]
    pub fn tracked_metrics(&self) -> &[String] {
        &self.metrics
    }

    /// Get export target.
    #[must_use]
    pub fn export_target(&self) -> Option<&str> {
        self.export_target.as_deref()
    }

    /// Get the latest value for a given metric name.
    #[must_use]
    pub fn latest(&self, metric_name: &str) -> Option<f64> {
        self.collected
            .iter()
            .rev()
            .find(|m| m.name == metric_name)
            .map(|m| m.value)
    }

    /// Clear all collected metrics.
    pub fn clear(&mut self) {
        self.collected.clear();
    }
}
