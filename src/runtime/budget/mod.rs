use std::fmt;

use super::provider::Usage;

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Cost calculation
// ---------------------------------------------------------------------------

/// Per-million-token pricing for a model.
#[derive(Debug, Clone, Copy)]
struct ModelPricing {
    /// Cost per 1M input tokens, in cents.
    input_cents_per_m: u64,
    /// Cost per 1M output tokens, in cents.
    output_cents_per_m: u64,
}

/// Default pricing for unknown models (conservative estimate).
const DEFAULT_PRICING: ModelPricing = ModelPricing {
    input_cents_per_m: 500,
    output_cents_per_m: 1500,
};

/// Look up pricing for a model.
fn pricing_for(model: &str) -> ModelPricing {
    let lower = model.to_lowercase();

    if lower.contains("gpt-4o-mini") {
        return ModelPricing {
            input_cents_per_m: 15,
            output_cents_per_m: 60,
        };
    }
    if lower.contains("gpt-4o") {
        return ModelPricing {
            input_cents_per_m: 250,
            output_cents_per_m: 1000,
        };
    }
    if lower.contains("gpt-4") {
        return ModelPricing {
            input_cents_per_m: 3000,
            output_cents_per_m: 6000,
        };
    }
    if lower.contains("gpt-3.5") {
        return ModelPricing {
            input_cents_per_m: 50,
            output_cents_per_m: 150,
        };
    }
    if lower.contains("claude") {
        return ModelPricing {
            input_cents_per_m: 300,
            output_cents_per_m: 1500,
        };
    }

    DEFAULT_PRICING
}

/// Calculate the cost in cents for a given model and token usage.
///
/// Uses approximate per-token pricing. Returns cost rounded up to the nearest cent.
#[must_use]
pub fn calculate_cost(model: &str, usage: &Usage) -> u64 {
    let pricing = pricing_for(model);

    let input_cost = usage.input_tokens * pricing.input_cents_per_m;
    let output_cost = usage.output_tokens * pricing.output_cents_per_m;
    let total = input_cost + output_cost;

    if total == 0 {
        return 0;
    }

    // Divide by 1M tokens, rounding up
    total.div_ceil(1_000_000)
}

// ---------------------------------------------------------------------------
// Budget tracking
// ---------------------------------------------------------------------------

/// Error when the budget has been exceeded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetExceeded {
    pub spent_cents: u64,
    pub limit_cents: u64,
}

impl fmt::Display for BudgetExceeded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "budget exceeded: spent {} cents of {} cent limit",
            self.spent_cents, self.limit_cents
        )
    }
}

impl std::error::Error for BudgetExceeded {}

/// Tracks spending against a budget limit.
#[derive(Debug, Clone)]
pub struct BudgetTracker {
    spent_cents: u64,
    limit_cents: u64,
}

impl BudgetTracker {
    /// Create a new tracker with the given limit in cents.
    #[must_use]
    pub fn new(limit_cents: u64) -> Self {
        Self {
            spent_cents: 0,
            limit_cents,
        }
    }

    /// Record a cost. Returns `Err` if total spending would exceed the budget.
    ///
    /// # Errors
    /// Returns `BudgetExceeded` if the new total exceeds the limit.
    pub fn record_usage(&mut self, cost_cents: u64) -> Result<(), BudgetExceeded> {
        let new_total = self.spent_cents + cost_cents;
        if new_total > self.limit_cents {
            return Err(BudgetExceeded {
                spent_cents: new_total,
                limit_cents: self.limit_cents,
            });
        }
        self.spent_cents = new_total;
        Ok(())
    }

    /// How many cents remain before the budget is exhausted.
    #[must_use]
    pub fn remaining_cents(&self) -> u64 {
        self.limit_cents.saturating_sub(self.spent_cents)
    }

    /// Whether the budget has been exceeded.
    #[must_use]
    pub fn is_exceeded(&self) -> bool {
        self.spent_cents > self.limit_cents
    }

    /// Current total spent.
    #[must_use]
    pub fn spent_cents(&self) -> u64 {
        self.spent_cents
    }

    /// The budget limit.
    #[must_use]
    pub fn limit_cents(&self) -> u64 {
        self.limit_cents
    }
}
