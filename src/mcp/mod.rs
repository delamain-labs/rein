//! Model Context Protocol (MCP) server for Rein.
//!
//! Exposes Rein's capabilities as MCP tools that AI assistants can call:
//! - `rein_validate` — validate a .rein policy
//! - `rein_explain` — explain a policy in plain language
//! - `rein_fmt` — format .rein source
//! - `rein_list_agents` — list agents in a policy
//! - `rein_list_workflows` — list workflows in a policy

use serde::{Deserialize, Serialize};

use crate::parser::parse;
use crate::validator;

#[cfg(test)]
mod tests;

/// MCP tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// MCP tool call result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResult {
    #[serde(rename = "type")]
    pub result_type: String,
    pub text: String,
}

/// List all available MCP tools.
#[must_use]
pub fn list_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "rein_validate".to_string(),
            description: "Validate a .rein policy file. Returns parse errors and validation diagnostics.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "The .rein source code to validate" },
                    "strict": { "type": "boolean", "description": "Enable strict mode (warn on unenforced features)", "default": false }
                },
                "required": ["source"]
            }),
        },
        McpTool {
            name: "rein_explain".to_string(),
            description: "Explain what a .rein policy defines in plain language.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "The .rein source code to explain" }
                },
                "required": ["source"]
            }),
        },
        McpTool {
            name: "rein_fmt".to_string(),
            description: "Format .rein source code to canonical style.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "The .rein source code to format" }
                },
                "required": ["source"]
            }),
        },
        McpTool {
            name: "rein_list_agents".to_string(),
            description: "List all agents defined in a .rein policy with their capabilities and budgets.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "The .rein source code to analyze" }
                },
                "required": ["source"]
            }),
        },
        McpTool {
            name: "rein_list_workflows".to_string(),
            description: "List all workflows defined in a .rein policy with their steps.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "The .rein source code to analyze" }
                },
                "required": ["source"]
            }),
        },
    ]
}

/// Call an MCP tool by name with the given arguments.
#[must_use]
pub fn call_tool(name: &str, args: &serde_json::Value) -> McpResult {
    let source = args
        .get("source")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    match name {
        "rein_validate" => tool_validate(source, args.get("strict").and_then(serde_json::Value::as_bool).unwrap_or(false)),
        "rein_explain" => tool_explain(source),
        "rein_fmt" => tool_fmt(source),
        "rein_list_agents" => tool_list_agents(source),
        "rein_list_workflows" => tool_list_workflows(source),
        _ => McpResult {
            result_type: "text".to_string(),
            text: format!("Unknown tool: {name}"),
        },
    }
}

fn tool_validate(source: &str, strict: bool) -> McpResult {
    let file = match parse(source) {
        Ok(f) => f,
        Err(e) => {
            return McpResult {
                result_type: "text".to_string(),
                text: format!("Parse error: {}", e.message),
            };
        }
    };

    let mut diags = validator::validate(&file);
    if strict {
        diags.extend(validator::strict::check_unenforced(&file));
    }

    if diags.is_empty() {
        McpResult {
            result_type: "text".to_string(),
            text: "✓ Valid".to_string(),
        }
    } else {
        let messages: Vec<String> = diags.iter().map(|d| format!("[{}] {}", d.code, d.message)).collect();
        McpResult {
            result_type: "text".to_string(),
            text: messages.join("\n"),
        }
    }
}

fn tool_explain(source: &str) -> McpResult {
    let file = match parse(source) {
        Ok(f) => f,
        Err(e) => {
            return McpResult {
                result_type: "text".to_string(),
                text: format!("Parse error: {}", e.message),
            };
        }
    };

    let mut lines = Vec::new();

    if !file.agents.is_empty() {
        lines.push(format!("Agents ({}):", file.agents.len()));
        for a in &file.agents {
            let model = a.model.as_ref().map_or("default".to_string(), |m| format!("{m:?}"));
            lines.push(format!("  • {} (model: {model})", a.name));
            if !a.can.is_empty() {
                let caps: Vec<String> = a.can.iter().map(|c| format!("{}.{}", c.namespace, c.action)).collect();
                lines.push(format!("    can: {}", caps.join(", ")));
            }
            if !a.cannot.is_empty() {
                let caps: Vec<String> = a.cannot.iter().map(|c| format!("{}.{}", c.namespace, c.action)).collect();
                lines.push(format!("    cannot: {}", caps.join(", ")));
            }
        }
    }

    if !file.workflows.is_empty() {
        lines.push(format!("\nWorkflows ({}):", file.workflows.len()));
        for w in &file.workflows {
            lines.push(format!("  • {} (trigger: {}, steps: {})", w.name, w.trigger, w.steps.len()));
            for s in &w.steps {
                lines.push(format!("    → {} (agent: {})", s.name, s.agent));
            }
        }
    }

    if lines.is_empty() {
        lines.push("Empty policy (no agents or workflows defined).".to_string());
    }

    McpResult {
        result_type: "text".to_string(),
        text: lines.join("\n"),
    }
}

fn tool_fmt(source: &str) -> McpResult {
    // Re-parse and re-emit. In production this would use the formatter.
    // For now, validate and return the source unchanged if valid.
    match parse(source) {
        Ok(_) => McpResult {
            result_type: "text".to_string(),
            text: source.to_string(),
        },
        Err(e) => McpResult {
            result_type: "text".to_string(),
            text: format!("Cannot format: {}", e.message),
        },
    }
}

fn tool_list_agents(source: &str) -> McpResult {
    let file = match parse(source) {
        Ok(f) => f,
        Err(e) => {
            return McpResult {
                result_type: "text".to_string(),
                text: format!("Parse error: {}", e.message),
            };
        }
    };

    if file.agents.is_empty() {
        return McpResult {
            result_type: "text".to_string(),
            text: "No agents defined.".to_string(),
        };
    }

    let agents: Vec<serde_json::Value> = file
        .agents
        .iter()
        .map(|a| {
            serde_json::json!({
                "name": a.name,
                "can": a.can.iter().map(|c| format!("{}.{}", c.namespace, c.action)).collect::<Vec<_>>(),
                "cannot": a.cannot.iter().map(|c| format!("{}.{}", c.namespace, c.action)).collect::<Vec<_>>(),
                "has_budget": a.budget.is_some(),
                "has_guardrails": a.guardrails.is_some(),
            })
        })
        .collect();

    McpResult {
        result_type: "text".to_string(),
        text: serde_json::to_string_pretty(&agents).unwrap_or_default(),
    }
}

fn tool_list_workflows(source: &str) -> McpResult {
    let file = match parse(source) {
        Ok(f) => f,
        Err(e) => {
            return McpResult {
                result_type: "text".to_string(),
                text: format!("Parse error: {}", e.message),
            };
        }
    };

    if file.workflows.is_empty() {
        return McpResult {
            result_type: "text".to_string(),
            text: "No workflows defined.".to_string(),
        };
    }

    let workflows: Vec<serde_json::Value> = file
        .workflows
        .iter()
        .map(|w| {
            let steps: Vec<serde_json::Value> = w
                .steps
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "name": s.name,
                        "agent": s.agent,
                        "has_when": s.when.is_some(),
                    })
                })
                .collect();
            serde_json::json!({
                "name": w.name,
                "trigger": w.trigger,
                "steps": steps,
            })
        })
        .collect();

    McpResult {
        result_type: "text".to_string(),
        text: serde_json::to_string_pretty(&workflows).unwrap_or_default(),
    }
}
