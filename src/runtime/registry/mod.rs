//! Tool registry client for discovering and fetching tool definitions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(test)]
mod tests;

/// A tool definition in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub namespace: String,
    pub version: String,
    pub description: String,
    /// Tool endpoint URL or command.
    pub endpoint: Option<String>,
    /// Input schema (JSON Schema).
    pub input_schema: Option<serde_json::Value>,
    /// Output schema (JSON Schema).
    pub output_schema: Option<serde_json::Value>,
}

/// In-memory tool registry.
#[derive(Debug, Default)]
pub struct ToolRegistry {
    tools: HashMap<String, ToolDef>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool.
    pub fn register(&mut self, tool: ToolDef) {
        let key = format!("{}.{}", tool.namespace, tool.name);
        self.tools.insert(key, tool);
    }

    /// Look up a tool by namespace.action.
    pub fn get(&self, qualified_name: &str) -> Option<&ToolDef> {
        self.tools.get(qualified_name)
    }

    /// List all tools in a namespace.
    pub fn list_namespace(&self, namespace: &str) -> Vec<&ToolDef> {
        self.tools
            .values()
            .filter(|t| t.namespace == namespace)
            .collect()
    }

    /// List all registered tools.
    pub fn list_all(&self) -> Vec<&ToolDef> {
        self.tools.values().collect()
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}
