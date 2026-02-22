use crate::ast::FleetDef;

#[cfg(test)]
mod tests;

/// Runtime state for a fleet of agents.
#[derive(Debug)]
pub struct Fleet {
    name: String,
    agents: Vec<String>,
    policy: Option<String>,
    budget_cents: Option<u64>,
    min_instances: u32,
    max_instances: u32,
    active_instances: u32,
}

/// Events emitted by fleet management.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FleetEvent {
    ScaledUp { from: u32, to: u32 },
    ScaledDown { from: u32, to: u32 },
    AtCapacity { current: u32, max: u32 },
    AtMinimum { current: u32, min: u32 },
}

impl Fleet {
    /// Create from a parsed fleet definition.
    #[must_use]
    pub fn from_def(def: &FleetDef) -> Self {
        let (min, max) = def.scaling.as_ref().map_or((1, 1), |s| (s.min, s.max));
        Self {
            name: def.name.clone(),
            agents: def.agents.clone(),
            policy: def.policy.clone(),
            budget_cents: def.budget,
            min_instances: min,
            max_instances: max,
            active_instances: min,
        }
    }

    /// Get the fleet name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get agent names in this fleet.
    #[must_use]
    pub fn agents(&self) -> &[String] {
        &self.agents
    }

    /// Get current active instance count.
    #[must_use]
    pub fn active_instances(&self) -> u32 {
        self.active_instances
    }

    /// Scale up by one instance. Returns event or None if at max.
    pub fn scale_up(&mut self) -> Option<FleetEvent> {
        if self.active_instances >= self.max_instances {
            return Some(FleetEvent::AtCapacity {
                current: self.active_instances,
                max: self.max_instances,
            });
        }
        let from = self.active_instances;
        self.active_instances += 1;
        Some(FleetEvent::ScaledUp {
            from,
            to: self.active_instances,
        })
    }

    /// Scale down by one instance. Returns event or None if at min.
    pub fn scale_down(&mut self) -> Option<FleetEvent> {
        if self.active_instances <= self.min_instances {
            return Some(FleetEvent::AtMinimum {
                current: self.active_instances,
                min: self.min_instances,
            });
        }
        let from = self.active_instances;
        self.active_instances -= 1;
        Some(FleetEvent::ScaledDown {
            from,
            to: self.active_instances,
        })
    }

    /// Get the policy name, if any.
    #[must_use]
    pub fn policy(&self) -> Option<&str> {
        self.policy.as_deref()
    }

    /// Get budget in cents, if any.
    #[must_use]
    pub fn budget_cents(&self) -> Option<u64> {
        self.budget_cents
    }
}
