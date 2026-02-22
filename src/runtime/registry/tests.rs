use super::*;

fn make_tool(ns: &str, name: &str) -> ToolDef {
    ToolDef {
        name: name.to_string(),
        namespace: ns.to_string(),
        version: "1.0.0".to_string(),
        description: format!("{ns}.{name}"),
        endpoint: None,
        input_schema: None,
        output_schema: None,
    }
}

#[test]
fn register_and_get() {
    let mut reg = ToolRegistry::new();
    reg.register(make_tool("zendesk", "read_ticket"));
    let tool = reg.get("zendesk.read_ticket").unwrap();
    assert_eq!(tool.name, "read_ticket");
}

#[test]
fn list_namespace() {
    let mut reg = ToolRegistry::new();
    reg.register(make_tool("zendesk", "read_ticket"));
    reg.register(make_tool("zendesk", "refund"));
    reg.register(make_tool("stripe", "charge"));
    assert_eq!(reg.list_namespace("zendesk").len(), 2);
    assert_eq!(reg.list_namespace("stripe").len(), 1);
}

#[test]
fn missing_tool() {
    let reg = ToolRegistry::new();
    assert!(reg.get("nonexistent.tool").is_none());
}
