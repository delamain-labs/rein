//! Abstract syntax tree types for the Rein language.

use serde::{Deserialize, Serialize};

mod agent;
mod approval;
mod channel;
mod circuit_breaker;
mod consensus;
mod escalate;
mod eval;
mod fleet;
mod import;
mod memory;
mod observe;
mod pipe;
mod policy;
mod provider;
mod scenario;
mod schedule;
mod secrets;
mod types;
mod value;
mod workflow;

pub use agent::*;
pub use approval::*;
pub use channel::*;
pub use circuit_breaker::*;
pub use consensus::*;
pub use escalate::*;
pub use eval::*;
pub use fleet::*;
pub use import::*;
pub use memory::*;
pub use observe::*;
pub use pipe::*;
pub use policy::*;
pub use provider::*;
pub use scenario::*;
pub use schedule::*;
pub use secrets::*;
pub use types::*;
pub use value::*;
pub use workflow::*;

/// Byte-offset span in the source file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

// ---------------------------------------------------------------------------
// Top-level file
// ---------------------------------------------------------------------------

/// Top-level parsed file — provider, agent, and workflow definitions.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ReinFile {
    pub imports: Vec<ImportDef>,
    pub defaults: Option<DefaultsDef>,
    pub providers: Vec<ProviderDef>,
    pub tools: Vec<ToolDef>,
    pub archetypes: Vec<ArchetypeDef>,
    pub agents: Vec<AgentDef>,
    pub workflows: Vec<WorkflowDef>,
    pub types: Vec<TypeDef>,
    pub policies: Vec<PolicyDef>,
    pub observes: Vec<ObserveDef>,
    pub fleets: Vec<FleetDef>,
    pub channels: Vec<ChannelDef>,
    pub circuit_breakers: Vec<CircuitBreakerDef>,
    pub evals: Vec<EvalDef>,
    pub memories: Vec<MemoryDef>,
    pub secrets: Vec<SecretsDef>,
    pub consensus_blocks: Vec<ConsensusDef>,
    pub scenarios: Vec<ScenarioDef>,
}

#[cfg(test)]
mod tests;
