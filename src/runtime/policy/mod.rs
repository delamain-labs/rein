use crate::ast::{PolicyDef, WhenExpr, WhenValue};

#[cfg(test)]
mod tests;

/// Tracks the current trust tier for an agent and enforces
/// tier-based capability restrictions.
#[derive(Debug)]
pub struct PolicyEngine {
    tiers: Vec<TierState>,
    current_tier_index: usize,
}

/// Runtime state for a single policy tier.
#[derive(Debug, Clone)]
pub struct TierState {
    pub name: String,
    /// The metric name referenced in the `promote when` condition.
    pub promote_metric: Option<String>,
    /// The threshold value for promotion (parsed from percentage).
    pub promote_threshold: Option<f64>,
}

/// A promotion event when an agent advances to the next tier.
#[derive(Debug, Clone)]
pub struct PromotionEvent {
    pub from_tier: String,
    pub to_tier: String,
    pub reason: String,
}

/// A demotion event when an agent drops back a tier.
#[derive(Debug, Clone)]
pub struct DemotionEvent {
    pub from_tier: String,
    pub to_tier: String,
    pub reason: String,
}

impl PolicyEngine {
    /// Create from a parsed policy definition.
    #[must_use]
    pub fn from_def(def: &PolicyDef) -> Self {
        let tiers = def
            .tiers
            .iter()
            .map(|t| {
                let (metric, threshold) = t
                    .promote_when
                    .as_ref()
                    .map_or((None, None), extract_simple_comparison);
                TierState {
                    name: t.name.clone(),
                    promote_metric: metric,
                    promote_threshold: threshold,
                }
            })
            .collect();
        Self {
            tiers,
            current_tier_index: 0,
        }
    }

    /// Get the name of the current tier.
    #[must_use]
    pub fn current_tier(&self) -> &str {
        self.tiers
            .get(self.current_tier_index)
            .map_or("unknown", |t| &t.name)
    }

    /// Get the zero-based index of the current tier.
    #[must_use]
    pub fn current_tier_index(&self) -> usize {
        self.current_tier_index
    }

    /// Check if the agent is at the highest tier.
    #[must_use]
    pub fn is_max_tier(&self) -> bool {
        self.current_tier_index >= self.tiers.len().saturating_sub(1)
    }

    /// Evaluate metrics and promote if the current tier's condition is met.
    /// Returns `Some(PromotionEvent)` if a promotion occurred.
    pub fn evaluate_promotion(&mut self, metrics: &[(String, f64)]) -> Option<PromotionEvent> {
        if self.is_max_tier() {
            return None;
        }

        let tier = &self.tiers[self.current_tier_index];
        let should_promote = match (&tier.promote_metric, tier.promote_threshold) {
            (Some(metric_name), Some(threshold)) => metrics
                .iter()
                .find(|(name, _)| name == metric_name)
                .is_some_and(|(_, value)| *value >= threshold),
            _ => false,
        };

        if should_promote {
            let from = self.tiers[self.current_tier_index].name.clone();
            self.current_tier_index += 1;
            let to = self.tiers[self.current_tier_index].name.clone();
            Some(PromotionEvent {
                from_tier: from,
                to_tier: to,
                reason: "Promotion threshold met".to_string(),
            })
        } else {
            None
        }
    }

    /// Total number of tiers.
    #[must_use]
    pub fn tier_count(&self) -> usize {
        self.tiers.len()
    }

    /// Demote the agent by one tier. Returns the demotion event, or None
    /// if already at the lowest tier.
    pub fn demote(&mut self, reason: &str) -> Option<DemotionEvent> {
        if self.current_tier_index == 0 {
            return None;
        }
        let from = self.tiers[self.current_tier_index].name.clone();
        self.current_tier_index -= 1;
        let to = self.tiers[self.current_tier_index].name.clone();
        Some(DemotionEvent {
            from_tier: from,
            to_tier: to,
            reason: reason.to_string(),
        })
    }
}

/// Extract metric name and threshold from a simple `WhenExpr::Comparison`.
/// Returns `(Some(metric), Some(threshold))` for simple comparisons,
/// `(None, None)` for complex/compound expressions.
fn extract_simple_comparison(expr: &WhenExpr) -> (Option<String>, Option<f64>) {
    match expr {
        WhenExpr::Comparison(cmp) => {
            let metric = Some(cmp.field.clone());
            let threshold = match &cmp.value {
                WhenValue::Percent(s) | WhenValue::Number(s) => s.parse::<f64>().ok(),
                #[allow(clippy::cast_precision_loss)]
                WhenValue::Currency { amount, .. } => Some(*amount as f64),
                WhenValue::String(_) | WhenValue::Ident(_) => None,
            };
            (metric, threshold)
        }
        WhenExpr::And(_) | WhenExpr::Or(_) => (None, None),
    }
}
