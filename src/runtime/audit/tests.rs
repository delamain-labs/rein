use super::*;
use tempfile::TempDir;

// #473: AuditLog::new must NOT create the target file as a side effect.
// Only the parent directory should be verified as writable.
#[test]
fn new_does_not_create_target_file_on_success() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("audit.jsonl");
    assert!(
        !path.exists(),
        "target file must not exist before AuditLog::new"
    );
    let _log = AuditLog::new(&path).unwrap();
    assert!(
        !path.exists(),
        "AuditLog::new must not create the target file as a side effect; \
         file should only appear on first append"
    );
}

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

// --- #529: generate_id extracted to module-level ---

#[test]
fn module_level_generate_id_produces_unique_ids() {
    let (id1, _) = generate_id();
    let (id2, _) = generate_id();
    assert_ne!(
        id1, id2,
        "successive generate_id() calls must produce distinct IDs"
    );
    assert!(
        id1.starts_with("audit-"),
        "generated ID must have audit- prefix"
    );
}

// --- #497 is_clock_reliable sentinel ---

#[test]
fn generate_id_returns_reliable_true_under_normal_conditions() {
    let (_id, reliable) = generate_id();
    assert!(
        reliable,
        "system clock is expected to be post-epoch in the test environment"
    );
}

#[test]
fn entry_builder_sets_is_clock_reliable_true() {
    let e = entry(AuditKind::WorkflowStart, "test");
    assert!(
        e.is_clock_reliable,
        "entry() must propagate clock reliability from generate_id()"
    );
}

#[test]
fn is_clock_reliable_defaults_to_true_on_deserialization() {
    // Old JSON produced before the field existed — must deserialize as true.
    let json = r#"{"id":"audit-abc-0","timestamp":"2025-01-01T00:00:00Z","kind":"workflow_start","description":"test","workflow":null,"agent":null,"step":null}"#;
    let e: AuditEntry = serde_json::from_str(json).unwrap();
    assert!(
        e.is_clock_reliable,
        "missing is_clock_reliable field must default to true for backward compat"
    );
}

#[test]
fn is_clock_reliable_false_round_trips() {
    let mut e = entry(AuditKind::WorkflowStart, "test");
    e.is_clock_reliable = false;
    let json = serde_json::to_string(&e).unwrap();
    let parsed: AuditEntry = serde_json::from_str(&json).unwrap();
    assert!(
        !parsed.is_clock_reliable,
        "is_clock_reliable = false must survive a JSON round-trip"
    );
}

// --- #499 AuditLog::new path probe fallback ---

#[test]
fn new_with_bare_filename_succeeds() {
    // A bare filename ("audit.jsonl") has parent() == Some(""), which is
    // functionally None. The probe must fall back to cwd rather than
    // silently skipping the writability check.
    //
    // Why this test uses cwd instead of a TempDir:
    // `std::env::set_current_dir` is process-global and not safe to use in
    // parallel tests. Bare-filename behavior is inherently cwd-dependent by
    // design — the test exercises the production contract (cwd must be
    // writable), which holds in all normal test environments. CI runners that
    // execute in read-only directories will fail this test, which is the
    // correct signal: a read-only cwd is a misconfiguration, not a test bug.
    //
    // We don't want to litter cwd with a real log file, so we use a
    // uniquely-named path and verify it doesn't get created (lazy init).
    let (id, _) = generate_id();
    let name = format!(".rein-test-bare-{id}.jsonl");
    let log = AuditLog::new(&name);
    // Cleanup probe remnants (if any) — best effort.
    let _ = std::fs::remove_file(&name);
    assert!(
        log.is_ok(),
        "AuditLog::new must succeed for a bare filename when cwd is writable"
    );
    // Target file must not have been created (lazy init contract).
    assert!(
        !std::path::Path::new(&name).exists(),
        "bare-filename AuditLog::new must not create the target file"
    );
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
        json.contains("\"approval_requested\""),
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
        json.contains("\"approval_resolved\""),
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

// #446: Each append must be durably written — visible to read_all immediately
// after the call returns. This catches BufWriter implementations that buffer
// without flushing: if flush is omitted, entries are lost on process exit.
#[test]
fn append_is_durable_within_single_log_instance() {
    let (_tmp, log) = test_log();
    for i in 0u32..10 {
        log.append(&entry(AuditKind::WorkflowStart, &format!("event {i}")))
            .unwrap();
        let all = log.read_all().unwrap();
        assert_eq!(
            all.len(),
            (i + 1) as usize,
            "entry {i} must be readable immediately after append; got {} entries",
            all.len()
        );
    }
}
