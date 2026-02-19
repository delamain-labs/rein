use super::permissions::ToolRegistry;
use super::ToolCall;

#[cfg(test)]
mod tests;

/// The result of intercepting a tool call.
#[derive(Debug, Clone, PartialEq)]
pub enum InterceptResult {
    /// The tool call is allowed with no monetary constraint.
    Allowed,
    /// The tool call is allowed but capped at a monetary amount.
    CappedAt {
        /// Maximum amount in cents.
        max_cents: u64,
        currency: String,
    },
    /// The tool call is denied.
    Denied {
        /// Human-readable reason.
        reason: String,
    },
}

/// Intercepts tool calls and checks them against a permission registry.
///
/// Sits between the LLM's requested tool calls and the actual tool executor,
/// enforcing the `.rein` file's `can`/`cannot` rules.
#[derive(Debug)]
pub struct ToolInterceptor<'a> {
    registry: &'a ToolRegistry,
}

impl<'a> ToolInterceptor<'a> {
    /// Create a new interceptor backed by the given permission registry.
    #[must_use]
    pub fn new(registry: &'a ToolRegistry) -> Self {
        Self { registry }
    }

    /// Check whether a tool call is permitted.
    ///
    /// Returns `Allowed` or `CappedAt` for permitted calls, `Denied` otherwise.
    #[must_use]
    pub fn intercept(&self, tool_call: &ToolCall) -> InterceptResult {
        match self.registry.check_permission(&tool_call.namespace, &tool_call.action) {
            Ok(()) => {
                // Check for monetary cap
                if let Some(cap) = self.registry.monetary_cap(&tool_call.namespace, &tool_call.action) {
                    InterceptResult::CappedAt {
                        max_cents: cap.amount,
                        currency: cap.currency.clone(),
                    }
                } else {
                    InterceptResult::Allowed
                }
            }
            Err(denied) => InterceptResult::Denied {
                reason: denied.reason,
            },
        }
    }
}
