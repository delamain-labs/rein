use super::*;

#[test]
fn restricted_denies_everything() {
    let policy = SandboxPolicy::restricted("agent1");
    assert!(policy.check_filesystem().is_err());
    assert!(policy.check_env().is_err());
    assert!(policy.check_network().is_err());
}

#[test]
fn restricted_allows_any_tool_when_empty() {
    let policy = SandboxPolicy::restricted("agent1");
    // Empty allowed_tools means all tools are allowed
    assert!(policy.check_tool("zendesk").is_ok());
}

#[test]
fn tool_allowlist() {
    let mut policy = SandboxPolicy::restricted("agent1");
    policy.allowed_tools.insert("zendesk".to_string());
    assert!(policy.check_tool("zendesk").is_ok());
    assert!(policy.check_tool("filesystem").is_err());
}

#[test]
fn allow_filesystem() {
    let mut policy = SandboxPolicy::restricted("agent1");
    policy.allow_filesystem = true;
    assert!(policy.check_filesystem().is_ok());
    assert!(policy.check_env().is_err()); // still denied
}

#[test]
fn violation_messages() {
    let v = SandboxViolation::ToolDenied {
        agent: "a".to_string(),
        tool: "t".to_string(),
    };
    assert!(v.to_string().contains("tool 't' denied"));
}
