use super::*;

const SAMPLE_POLICY: &str = r#"
    provider openai {
        model: openai
        key: env("OPENAI_KEY")
    }
    agent support {
        model: openai
        can [
            tickets.read
            tickets.respond
        ]
        cannot [
            data.delete
        ]
    }
    workflow support_flow {
        trigger: new_ticket
        step classify {
            agent: support
            goal: "Classify ticket"
        }
    }
"#;

#[test]
fn list_tools_returns_five() {
    let tools = list_tools();
    assert_eq!(tools.len(), 5);
}

#[test]
fn list_tools_has_validate() {
    let tools = list_tools();
    assert!(tools.iter().any(|t| t.name == "rein_validate"));
}

#[test]
fn validate_valid_source() {
    let args = serde_json::json!({ "source": SAMPLE_POLICY });
    let result = call_tool("rein_validate", &args);
    assert_eq!(result.text, "✓ Valid");
}

#[test]
fn validate_invalid_source() {
    let args = serde_json::json!({ "source": "agent { }" });
    let result = call_tool("rein_validate", &args);
    assert!(result.text.contains("error") || result.text.contains("Parse error"));
}

#[test]
fn explain_shows_agents_and_workflows() {
    let args = serde_json::json!({ "source": SAMPLE_POLICY });
    let result = call_tool("rein_explain", &args);
    assert!(result.text.contains("support"));
    assert!(result.text.contains("support_flow"));
}

#[test]
fn list_agents_returns_json() {
    let args = serde_json::json!({ "source": SAMPLE_POLICY });
    let result = call_tool("rein_list_agents", &args);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&result.text).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["name"], "support");
}

#[test]
fn list_workflows_returns_json() {
    let args = serde_json::json!({ "source": SAMPLE_POLICY });
    let result = call_tool("rein_list_workflows", &args);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&result.text).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["name"], "support_flow");
    assert_eq!(parsed[0]["steps"].as_array().unwrap().len(), 1);
}

#[test]
fn unknown_tool_returns_error() {
    let args = serde_json::json!({ "source": "" });
    let result = call_tool("nonexistent", &args);
    assert!(result.text.contains("Unknown tool"));
}

#[test]
fn fmt_valid_source_returns_source() {
    let source = "agent test { model: openai }";
    let args = serde_json::json!({ "source": source });
    let result = call_tool("rein_fmt", &args);
    assert_eq!(result.text, source);
}

#[test]
fn validate_strict_warns_on_guardrails() {
    let source = "agent safe {\n    model: openai\n    guardrails {\n        output_filter {\n            pii: redact\n        }\n    }\n}";
    let args = serde_json::json!({ "source": source, "strict": true });
    let result = call_tool("rein_validate", &args);
    assert!(result.text.contains("UNENFORCED") || result.text.contains("not enforced") || result.text.contains("guardrails"));
}
