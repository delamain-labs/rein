//! Per-agent sandbox isolation.
//!
//! Enforces process-level constraints: no filesystem access, no env vars,
//! no raw network. Each agent runs within a `SandboxPolicy`.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[cfg(test)]
mod tests;

/// Sandbox policy for an agent execution context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPolicy {
    /// Agent name this policy applies to.
    pub agent: String,
    /// Whether filesystem access is allowed.
    pub allow_filesystem: bool,
    /// Whether environment variable access is allowed.
    pub allow_env: bool,
    /// Whether raw network access is allowed.
    pub allow_network: bool,
    /// Allowed tool namespaces (e.g., "zendesk", "search").
    pub allowed_tools: HashSet<String>,
    /// Maximum execution time in seconds.
    pub max_execution_secs: Option<u64>,
    /// Maximum memory in bytes (advisory).
    pub max_memory_bytes: Option<u64>,
}

impl SandboxPolicy {
    /// Create a fully restricted sandbox (default).
    pub fn restricted(agent: impl Into<String>) -> Self {
        Self {
            agent: agent.into(),
            allow_filesystem: false,
            allow_env: false,
            allow_network: false,
            allowed_tools: HashSet::new(),
            max_execution_secs: Some(300),
            max_memory_bytes: None,
        }
    }

    /// Check if a tool call is permitted.
    pub fn check_tool(&self, namespace: &str) -> Result<(), SandboxViolation> {
        if self.allowed_tools.is_empty() || self.allowed_tools.contains(namespace) {
            Ok(())
        } else {
            Err(SandboxViolation::ToolDenied {
                agent: self.agent.clone(),
                tool: namespace.to_string(),
            })
        }
    }

    /// Check if filesystem access is permitted.
    pub fn check_filesystem(&self) -> Result<(), SandboxViolation> {
        if self.allow_filesystem {
            Ok(())
        } else {
            Err(SandboxViolation::FilesystemDenied {
                agent: self.agent.clone(),
            })
        }
    }

    /// Check if env access is permitted.
    pub fn check_env(&self) -> Result<(), SandboxViolation> {
        if self.allow_env {
            Ok(())
        } else {
            Err(SandboxViolation::EnvDenied {
                agent: self.agent.clone(),
            })
        }
    }

    /// Check if network access is permitted.
    pub fn check_network(&self) -> Result<(), SandboxViolation> {
        if self.allow_network {
            Ok(())
        } else {
            Err(SandboxViolation::NetworkDenied {
                agent: self.agent.clone(),
            })
        }
    }
}

/// Sandbox policy violation.
#[derive(Debug, Clone, PartialEq)]
pub enum SandboxViolation {
    ToolDenied { agent: String, tool: String },
    FilesystemDenied { agent: String },
    EnvDenied { agent: String },
    NetworkDenied { agent: String },
}

impl std::fmt::Display for SandboxViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ToolDenied { agent, tool } => {
                write!(f, "agent '{agent}': tool '{tool}' denied by sandbox")
            }
            Self::FilesystemDenied { agent } => {
                write!(f, "agent '{agent}': filesystem access denied by sandbox")
            }
            Self::EnvDenied { agent } => {
                write!(f, "agent '{agent}': env var access denied by sandbox")
            }
            Self::NetworkDenied { agent } => {
                write!(f, "agent '{agent}': network access denied by sandbox")
            }
        }
    }
}

impl std::error::Error for SandboxViolation {}
