use super::*;
use tempfile::TempDir;

fn test_log() -> (TempDir, AuditLog) {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("audit.jsonl");
    let log = AuditLog::new(&path).unwrap();
    (tmp, log)
}

#[test]
fn append_and_read() {
    let (_tmp, log) = test_log();
    let e = entry(AuditKind::WorkflowStart, "Started pipeline");
    log.append(&e).unwrap();
    let entries = log.read_all().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].description, "Started pipeline");
}

#[test]
fn multiple_entries() {
    let (_tmp, log) = test_log();
    log.append(&entry(AuditKind::WorkflowStart, "start"))
        .unwrap();
    log.append(&entry(AuditKind::StepStart, "step 1")).unwrap();
    log.append(&entry(AuditKind::StepComplete, "step 1 done"))
        .unwrap();
    log.append(&entry(AuditKind::WorkflowComplete, "done"))
        .unwrap();

    let entries = log.read_all().unwrap();
    assert_eq!(entries.len(), 4);
}

#[test]
fn query_by_kind() {
    let (_tmp, log) = test_log();
    log.append(&entry(AuditKind::ToolCall, "called search"))
        .unwrap();
    log.append(&entry(AuditKind::GuardrailViolation, "PII detected"))
        .unwrap();
    log.append(&entry(AuditKind::ToolCall, "called write"))
        .unwrap();

    let tool_calls = log.query_by_kind(&AuditKind::ToolCall).unwrap();
    assert_eq!(tool_calls.len(), 2);
}

#[test]
fn query_by_workflow() {
    let (_tmp, log) = test_log();
    let mut e1 = entry(AuditKind::WorkflowStart, "start");
    e1.workflow = Some("pipeline".to_string());
    log.append(&e1).unwrap();

    let mut e2 = entry(AuditKind::WorkflowStart, "other");
    e2.workflow = Some("other_wf".to_string());
    log.append(&e2).unwrap();

    let results = log.query_by_workflow("pipeline").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].description, "start");
}

#[test]
fn empty_log_returns_empty_vec() {
    let (_tmp, log) = test_log();
    assert!(log.read_all().unwrap().is_empty());
}

#[test]
fn entry_builder() {
    let e = entry(AuditKind::Escalation, "escalated to human");
    assert!(e.id.starts_with("audit-"));
    assert_eq!(e.kind, AuditKind::Escalation);
    assert!(e.workflow.is_none());
}

#[test]
fn serialization_roundtrip() {
    let mut e = entry(AuditKind::Custom("deploy".to_string()), "deployed v2");
    e.workflow = Some("deploy_wf".to_string());
    e.agent = Some("deployer".to_string());
    e.metadata = serde_json::json!({"version": "2.0"});

    let json = serde_json::to_string(&e).unwrap();
    let parsed: AuditEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.kind, AuditKind::Custom("deploy".to_string()));
    assert_eq!(parsed.metadata["version"], "2.0");
}

// --- #358 Approval audit event kinds ---

#[test]
fn approval_requested_kind_serializes_correctly() {
    let e = entry(
        AuditKind::ApprovalRequested,
        "Approval requested for step deploy",
    );
    let json = serde_json::to_string(&e).unwrap();
    assert!(
        json.contains("approval_requested"),
        "kind must serialize to snake_case"
    );
}

#[test]
fn approval_resolved_kind_serializes_correctly() {
    let e = entry(
        AuditKind::ApprovalResolved,
        "Approval resolved for step deploy",
    );
    let json = serde_json::to_string(&e).unwrap();
    assert!(
        json.contains("approval_resolved"),
        "kind must serialize to snake_case"
    );
}

#[test]
fn query_by_approval_requested_kind() {
    let (_tmp, log) = test_log();
    log.append(&entry(AuditKind::WorkflowStart, "start"))
        .unwrap();
    log.append(&entry(AuditKind::ApprovalRequested, "req"))
        .unwrap();
    log.append(&entry(AuditKind::ApprovalResolved, "res"))
        .unwrap();

    let requested = log.query_by_kind(&AuditKind::ApprovalRequested).unwrap();
    assert_eq!(requested.len(), 1);
    let resolved = log.query_by_kind(&AuditKind::ApprovalResolved).unwrap();
    assert_eq!(resolved.len(), 1);
}
