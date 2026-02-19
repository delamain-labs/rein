use std::collections::HashMap;

use crate::ast::{AgentDef, Constraint};

/// Returned when a tool call is blocked by the registry.
#[derive(Debug, Clone, PartialEq)]
pub struct PermissionDenied {
    pub reason: String,
}

impl std::fmt::Display for PermissionDenied {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "permission denied: {}", self.reason)
    }
}

impl std::error::Error for PermissionDenied {}

/// A monetary cap attached to an allowed capability (`up to $<amount>`).
#[derive(Debug, Clone, PartialEq)]
pub struct MonetaryCap {
    /// Amount in the smallest currency unit (e.g. cents for USD).
    pub amount: u64,
    pub currency: String,
}

/// Internal classification of a tool's permission status.
#[derive(Debug)]
enum ToolEntry {
    Allowed { cap: Option<MonetaryCap> },
    Denied,
}

/// Registry of allowed/denied tools built from an [`AgentDef`]'s `can`/`cannot` lists.
///
/// Tools not present in either list are **default-denied**.
/// If a tool appears in both lists, `cannot` takes precedence.
#[derive(Debug)]
pub struct ToolRegistry {
    /// Key is `"namespace.action"`.
    tools: HashMap<String, ToolEntry>,
}

impl ToolRegistry {
    /// Build a registry from a parsed agent definition.
    pub fn from_agent(agent: &AgentDef) -> Self {
        let mut tools = HashMap::new();

        for cap in &agent.can {
            let monetary_cap = cap.constraint.as_ref().map(|c| match c {
                Constraint::MonetaryCap { amount, currency } => MonetaryCap {
                    amount: *amount,
                    currency: currency.clone(),
                },
            });
            tools.insert(
                Self::key(&cap.namespace, &cap.action),
                ToolEntry::Allowed { cap: monetary_cap },
            );
        }

        // `cannot` overrides `can` when both list the same tool.
        for cap in &agent.cannot {
            tools.insert(Self::key(&cap.namespace, &cap.action), ToolEntry::Denied);
        }

        Self { tools }
    }

    /// Check whether `namespace.action` is permitted.
    ///
    /// Returns `Ok(())` if the tool is in the `can` list.
    /// Returns `Err` if the tool is in the `cannot` list or in neither list.
    pub fn check_permission(&self, namespace: &str, action: &str) -> Result<(), PermissionDenied> {
        match self.tools.get(&Self::key(namespace, action)) {
            Some(ToolEntry::Allowed { .. }) => Ok(()),
            Some(ToolEntry::Denied) => Err(PermissionDenied {
                reason: format!("`{namespace}.{action}` is explicitly listed in the cannot block"),
            }),
            None => Err(PermissionDenied {
                reason: format!("`{namespace}.{action}` is not in the can list (default deny)"),
            }),
        }
    }

    /// Returns the monetary cap for an allowed tool, if one was declared.
    ///
    /// Returns `None` for denied or unknown tools, and for allowed tools with
    /// no constraint.
    pub fn monetary_cap(&self, namespace: &str, action: &str) -> Option<&MonetaryCap> {
        match self.tools.get(&Self::key(namespace, action)) {
            Some(ToolEntry::Allowed { cap }) => cap.as_ref(),
            _ => None,
        }
    }

    fn key(namespace: &str, action: &str) -> String {
        format!("{namespace}.{action}")
    }
}
