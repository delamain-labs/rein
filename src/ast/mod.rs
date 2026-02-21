//! Abstract syntax tree types for the Rein language.

use serde::{Deserialize, Serialize};

mod agent;
mod import;
mod provider;
mod types;
mod value;
mod workflow;

pub use agent::*;
pub use import::*;
pub use provider::*;
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReinFile {
    pub imports: Vec<ImportDef>,
    pub defaults: Option<DefaultsDef>,
    pub providers: Vec<ProviderDef>,
    pub tools: Vec<ToolDef>,
    pub agents: Vec<AgentDef>,
    pub workflows: Vec<WorkflowDef>,
    pub types: Vec<TypeDef>,
}

#[cfg(test)]
mod tests;
