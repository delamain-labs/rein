use crate::ast::{ConsensusDef, ConsensusStrategy};

#[cfg(test)]
mod tests;

/// The result of a consensus vote from one agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Vote {
    Agree,
    Disagree { reason: String },
}

/// The outcome of a consensus round.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsensusOutcome {
    /// Consensus reached: enough agents agreed.
    Reached { agreed: u32, total: u32 },
    /// Consensus failed: not enough agents agreed.
    Failed {
        agreed: u32,
        required: u32,
        total: u32,
    },
}

/// Evaluates consensus from a set of votes according to the strategy.
pub struct ConsensusEvaluator {
    strategy: ConsensusStrategy,
    required: Option<u32>,
    agent_count: u32,
}

impl ConsensusEvaluator {
    /// Create from a parsed consensus definition.
    #[must_use]
    pub fn from_def(def: &ConsensusDef) -> Self {
        let agent_count = u32::try_from(def.agents.len()).unwrap_or(u32::MAX);
        Self {
            strategy: def.strategy.clone(),
            required: def.require.as_ref().map(|r| r.required),
            agent_count,
        }
    }

    /// Evaluate a set of votes and return the outcome.
    #[must_use]
    pub fn evaluate(&self, votes: &[Vote]) -> ConsensusOutcome {
        let agreed =
            u32::try_from(votes.iter().filter(|v| **v == Vote::Agree).count()).unwrap_or(u32::MAX);
        let total = u32::try_from(votes.len()).unwrap_or(u32::MAX);
        let required = self.required_count();

        if agreed >= required {
            ConsensusOutcome::Reached { agreed, total }
        } else {
            ConsensusOutcome::Failed {
                agreed,
                required,
                total,
            }
        }
    }

    /// How many votes are required based on the strategy.
    #[must_use]
    pub fn required_count(&self) -> u32 {
        if let Some(explicit) = self.required {
            return explicit;
        }
        match self.strategy {
            ConsensusStrategy::Majority => (self.agent_count / 2) + 1,
            ConsensusStrategy::Unanimous | ConsensusStrategy::Custom(_) => self.agent_count,
        }
    }
}
