use std::collections::VecDeque;
use std::time::Instant;

#[cfg(test)]
mod tests;

/// The state a circuit breaker can be in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakerState {
    /// Normal operation: requests pass through.
    Closed,
    /// Failure threshold exceeded: requests are blocked.
    Open,
    /// Cooling off: one probe request is allowed to test recovery.
    HalfOpen,
}

/// A circuit breaker that tracks failures within a rolling time window
/// and trips open when the threshold is exceeded.
#[derive(Debug)]
pub struct CircuitBreaker {
    name: String,
    failure_threshold: u32,
    window: std::time::Duration,
    half_open_after: std::time::Duration,
    state: BreakerState,
    failures: VecDeque<Instant>,
    opened_at: Option<Instant>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker from parsed config.
    #[must_use]
    pub fn new(
        name: &str,
        failure_threshold: u32,
        window_minutes: u32,
        half_open_after_minutes: u32,
    ) -> Self {
        Self {
            name: name.to_string(),
            failure_threshold,
            window: std::time::Duration::from_secs(u64::from(window_minutes) * 60),
            half_open_after: std::time::Duration::from_secs(
                u64::from(half_open_after_minutes) * 60,
            ),
            state: BreakerState::Closed,
            failures: VecDeque::new(),
            opened_at: None,
        }
    }

    /// Create from an AST definition.
    #[must_use]
    pub fn from_def(def: &crate::ast::CircuitBreakerDef) -> Self {
        Self::new(
            &def.name,
            def.failure_threshold,
            def.window_minutes,
            def.half_open_after_minutes,
        )
    }

    /// Get the current state, transitioning from `Open` to `HalfOpen` if
    /// enough time has passed.
    #[must_use]
    pub fn state(&mut self) -> BreakerState {
        if self.state == BreakerState::Open
            && self
                .opened_at
                .is_some_and(|opened| opened.elapsed() >= self.half_open_after)
        {
            self.state = BreakerState::HalfOpen;
        }
        self.state
    }

    /// Check whether a request should be allowed through.
    /// Returns `Ok(())` if allowed, `Err(reason)` if blocked.
    ///
    /// # Errors
    /// Returns an error string when the circuit is open.
    pub fn check(&mut self) -> Result<(), String> {
        match self.state() {
            BreakerState::Closed | BreakerState::HalfOpen => Ok(()),
            BreakerState::Open => Err(format!(
                "Circuit breaker '{}' is open: {} failures in window exceeded threshold of {}",
                self.name,
                self.count_recent_failures(),
                self.failure_threshold
            )),
        }
    }

    /// Record a successful operation. Resets the breaker if half-open.
    pub fn record_success(&mut self) {
        if self.state == BreakerState::HalfOpen {
            self.state = BreakerState::Closed;
            self.failures.clear();
            self.opened_at = None;
        }
    }

    /// Record a failed operation. May trip the breaker open.
    pub fn record_failure(&mut self) {
        let now = Instant::now();
        self.failures.push_back(now);
        self.prune_old_failures(now);

        if self.state == BreakerState::HalfOpen {
            // Probe failed: go back to open.
            self.state = BreakerState::Open;
            self.opened_at = Some(now);
            return;
        }

        if self.count_recent_failures() >= self.failure_threshold {
            self.state = BreakerState::Open;
            self.opened_at = Some(now);
        }
    }

    /// Get the breaker name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Current failure count within the rolling window.
    #[must_use]
    pub fn failure_count(&self) -> u32 {
        self.count_recent_failures()
    }

    /// The configured failure threshold at which the breaker trips open.
    #[must_use]
    pub fn threshold(&self) -> u32 {
        self.failure_threshold
    }

    /// Count failures within the current window.
    fn count_recent_failures(&self) -> u32 {
        self.failures.len().try_into().unwrap_or(u32::MAX)
    }

    /// Remove failures older than the window.
    fn prune_old_failures(&mut self, now: Instant) {
        while let Some(&front) = self.failures.front() {
            if now.duration_since(front) > self.window {
                self.failures.pop_front();
            } else {
                break;
            }
        }
    }
}
