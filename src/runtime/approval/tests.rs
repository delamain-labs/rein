use super::*;
use crate::ast::{ApprovalDef, ApprovalKind, Span};

fn span() -> Span {
    Span { start: 0, end: 0 }
}

fn make_approval() -> ApprovalDef {
    ApprovalDef {
        kind: ApprovalKind::Approve,
        channel: "slack".to_string(),
        destination: "#approvals".to_string(),
        timeout: Some("4h".to_string()),
        mode: None,
        span: span(),
    }
}

#[tokio::test]
async fn auto_approve_returns_approved() {
    let handler = AutoApproveHandler;
    let approval = make_approval();
    let status = handler
        .request_approval("deploy", "Agent output here", &approval)
        .await;
    assert_eq!(status, ApprovalStatus::Approved);
}

#[tokio::test]
async fn auto_reject_returns_rejected() {
    let handler = AutoRejectHandler::new("policy violation");
    let approval = make_approval();
    let status = handler
        .request_approval("deploy", "Agent output here", &approval)
        .await;
    assert_eq!(
        status,
        ApprovalStatus::Rejected {
            reason: "policy violation".to_string()
        }
    );
}

#[test]
fn parse_timeout_hours() {
    assert_eq!(parse_timeout("4h"), Some(14400));
}

#[test]
fn parse_timeout_minutes() {
    assert_eq!(parse_timeout("30m"), Some(1800));
}

#[test]
fn parse_timeout_seconds() {
    assert_eq!(parse_timeout("60s"), Some(60));
}

#[test]
fn parse_timeout_bare_number() {
    assert_eq!(parse_timeout("3600"), Some(3600));
}

#[test]
fn parse_timeout_invalid() {
    assert_eq!(parse_timeout("abc"), None);
}

#[test]
fn approval_status_equality() {
    assert_eq!(ApprovalStatus::Approved, ApprovalStatus::Approved);
    assert_eq!(ApprovalStatus::TimedOut, ApprovalStatus::TimedOut);
    assert_eq!(ApprovalStatus::Pending, ApprovalStatus::Pending);
    assert_ne!(ApprovalStatus::Approved, ApprovalStatus::TimedOut);
}

#[test]
fn collaborate_approval_def() {
    let def = ApprovalDef {
        kind: ApprovalKind::Collaborate,
        channel: "dashboard".to_string(),
        destination: "/review".to_string(),
        timeout: None,
        mode: Some(crate::ast::CollaborationMode::Review),
        span: span(),
    };
    assert_eq!(def.kind, ApprovalKind::Collaborate);
    assert!(def.timeout.is_none());
    assert_eq!(def.mode, Some(crate::ast::CollaborationMode::Review));
}

// --- #308 Channel Routing Tests ---

fn make_approval_for_channel(channel: &str, destination: &str) -> ApprovalDef {
    ApprovalDef {
        kind: ApprovalKind::Approve,
        channel: channel.to_string(),
        destination: destination.to_string(),
        timeout: Some("1h".to_string()),
        mode: None,
        span: span(),
    }
}

#[test]
fn resolve_handler_cli_for_cli_channel() {
    // CliApprovalHandler reads from stdin so it cannot be exercised in a headless
    // test; we verify the dispatch path constructs without panic.
    let approval = make_approval_for_channel("cli", "");
    let _ = resolve_approval_handler(&approval);
}

#[test]
fn resolve_handler_empty_channel_falls_back_to_cli() {
    // An empty channel string is a misconfiguration — should fall back to CLI.
    let approval = make_approval_for_channel("", "");
    let _ = resolve_approval_handler(&approval);
}

#[test]
fn resolve_handler_cli_for_unknown_channel() {
    // Unknown channel types fall back to CLI with a warning.
    let approval = make_approval_for_channel("unknown_channel_xyz", "");
    let _ = resolve_approval_handler(&approval);
}

// #350: webhook must fail-closed — network errors must NOT auto-approve.
#[tokio::test]
async fn webhook_handler_rejects_on_network_failure() {
    let handler = WebhookApprovalHandler::new("http://localhost:0/nonexistent".to_string());
    let approval = make_approval_for_channel("webhook", "http://localhost:0/nonexistent");
    let status = handler
        .request_approval("deploy", "Agent output here", &approval)
        .await;
    assert!(
        matches!(status, ApprovalStatus::Rejected { .. }),
        "webhook network failure must reject, not approve; got {status:?}"
    );
}

// #350: non-2xx server error must reject, not approve.
#[tokio::test]
async fn webhook_handler_rejects_on_server_error() {
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    let url = format!("{}/approval", server.uri());
    let handler = WebhookApprovalHandler::new(url.clone());
    let approval = make_approval_for_channel("webhook", &url);
    let status = handler
        .request_approval("deploy", "Agent output here", &approval)
        .await;
    assert!(
        matches!(status, ApprovalStatus::Rejected { .. }),
        "webhook 500 must reject; got {status:?}"
    );
}

// #350: client error (4xx) must also reject — endpoint exists but denied access.
#[tokio::test]
async fn webhook_handler_rejects_on_client_error() {
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;
    let url = format!("{}/approval", server.uri());
    let handler = WebhookApprovalHandler::new(url.clone());
    let approval = make_approval_for_channel("webhook", &url);
    let status = handler
        .request_approval("deploy", "Agent output here", &approval)
        .await;
    assert!(
        matches!(status, ApprovalStatus::Rejected { .. }),
        "webhook 403 must reject; got {status:?}"
    );
}

// #350: 2xx response must still approve.
#[tokio::test]
async fn webhook_handler_approves_on_2xx() {
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    let url = format!("{}/approval", server.uri());
    let handler = WebhookApprovalHandler::new(url.clone());
    let approval = make_approval_for_channel("webhook", &url);
    let status = handler
        .request_approval("deploy", "Agent output here", &approval)
        .await;
    assert_eq!(status, ApprovalStatus::Approved);
}

#[tokio::test]
async fn slack_handler_falls_back_to_auto_approve_on_failure() {
    // Slack webhook POST will fail (invalid URL); should fall back gracefully.
    let handler = SlackApprovalHandler::new("http://localhost:0/nonexistent".to_string());
    let approval = make_approval_for_channel("slack", "http://localhost:0/nonexistent");
    let status = handler
        .request_approval("notify", "Agent output here", &approval)
        .await;
    assert_eq!(status, ApprovalStatus::Approved);
}

#[tokio::test]
async fn resolve_handler_returns_slack_for_slack_channel() {
    // Dispatch to Slack path; verify the resolved handler auto-approves on network failure.
    let approval = make_approval_for_channel("slack", "http://localhost:0/nonexistent");
    let handler = resolve_approval_handler(&approval);
    let status = handler
        .request_approval("deploy", "Agent output here", &approval)
        .await;
    assert_eq!(status, ApprovalStatus::Approved);
}

#[tokio::test]
async fn resolve_handler_returns_webhook_for_webhook_channel() {
    // Dispatch to webhook path; verify the resolved handler rejects on network failure (#350).
    let approval = make_approval_for_channel("webhook", "http://localhost:0/nonexistent");
    let handler = resolve_approval_handler(&approval);
    let status = handler
        .request_approval("deploy", "Agent output here", &approval)
        .await;
    assert!(
        matches!(status, ApprovalStatus::Rejected { .. }),
        "resolved webhook handler must reject on network failure; got {status:?}"
    );
}

// Handler that always returns TimedOut, for testing that path.
struct AutoTimedOutHandler;
#[async_trait::async_trait]
impl ApprovalHandler for AutoTimedOutHandler {
    async fn request_approval(
        &self,
        _step_name: &str,
        _agent_output: &str,
        _approval: &crate::ast::ApprovalDef,
    ) -> ApprovalStatus {
        ApprovalStatus::TimedOut
    }
}

// Handler that always returns Pending, for testing the async-deferred path.
struct AutoPendingHandler;
#[async_trait::async_trait]
impl ApprovalHandler for AutoPendingHandler {
    async fn request_approval(
        &self,
        _step_name: &str,
        _agent_output: &str,
        _approval: &crate::ast::ApprovalDef,
    ) -> ApprovalStatus {
        ApprovalStatus::Pending
    }
}

// --- #358 Approval Audit Events ---

#[tokio::test]
async fn auditing_handler_logs_approval_requested_and_resolved() {
    use crate::runtime::audit::{AuditKind, AuditLog};
    use std::sync::Arc;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let log = Arc::new(AuditLog::new(tmp.path().join("audit.jsonl")).unwrap());

    let handler = AuditingApprovalHandler::new(Arc::new(AutoApproveHandler), Arc::clone(&log));

    let approval = make_approval_for_channel("cli", "");
    let status = handler
        .request_approval("deploy", "Agent output", &approval)
        .await;

    assert_eq!(status, ApprovalStatus::Approved);

    let entries = log.read_all().unwrap();
    assert_eq!(
        entries.len(),
        2,
        "expected ApprovalRequested + ApprovalResolved"
    );

    assert_eq!(entries[0].kind, AuditKind::ApprovalRequested);
    assert!(entries[0].step.as_deref() == Some("deploy"));

    assert_eq!(entries[1].kind, AuditKind::ApprovalResolved);
    assert!(entries[1].step.as_deref() == Some("deploy"));
    assert_eq!(entries[1].metadata["decision"], "approved");
    assert!(
        entries[1].metadata["elapsed_ms"].is_number(),
        "elapsed_ms must be a numeric field in the resolved entry"
    );
}

#[tokio::test]
async fn auditing_handler_records_rejected_decision() {
    use crate::runtime::audit::{AuditKind, AuditLog};
    use std::sync::Arc;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let log = Arc::new(AuditLog::new(tmp.path().join("audit.jsonl")).unwrap());

    let handler = AuditingApprovalHandler::new(
        Arc::new(AutoRejectHandler::new("policy violation")),
        Arc::clone(&log),
    );

    let approval = make_approval_for_channel("cli", "");
    let status = handler
        .request_approval("review", "Agent output", &approval)
        .await;

    assert!(matches!(status, ApprovalStatus::Rejected { .. }));

    let entries = log.read_all().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[1].kind, AuditKind::ApprovalResolved);
    assert_eq!(entries[1].metadata["decision"], "rejected");
    assert_eq!(entries[1].metadata["reason"], "policy violation");
}

#[tokio::test]
async fn auditing_handler_records_channel_in_metadata() {
    use crate::runtime::audit::AuditLog;
    use std::sync::Arc;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let log = Arc::new(AuditLog::new(tmp.path().join("audit.jsonl")).unwrap());

    let handler = AuditingApprovalHandler::new(Arc::new(AutoApproveHandler), Arc::clone(&log));
    let approval = make_approval_for_channel("slack", "https://hooks.slack.com/fake");
    handler.request_approval("notify", "out", &approval).await;

    let entries = log.read_all().unwrap();
    assert_eq!(
        entries.len(),
        2,
        "expected ApprovalRequested + ApprovalResolved"
    );
    assert_eq!(entries[0].metadata["channel"], "slack");
    assert_eq!(entries[1].metadata["channel"], "slack");
}

#[tokio::test]
async fn auditing_handler_records_timed_out_decision() {
    use crate::runtime::audit::{AuditKind, AuditLog};
    use std::sync::Arc;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let log = Arc::new(AuditLog::new(tmp.path().join("audit.jsonl")).unwrap());

    let handler = AuditingApprovalHandler::new(Arc::new(AutoTimedOutHandler), Arc::clone(&log));
    let approval = make_approval_for_channel("cli", "");
    let status = handler
        .request_approval("timeout-step", "Agent output", &approval)
        .await;

    assert_eq!(status, ApprovalStatus::TimedOut);

    let entries = log.read_all().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[1].kind, AuditKind::ApprovalResolved);
    assert_eq!(
        entries[1].metadata["decision"], "timed_out",
        "timed_out decision must be recorded in metadata"
    );
}

#[tokio::test]
async fn auditing_handler_records_pending_decision() {
    use crate::runtime::audit::{AuditKind, AuditLog};
    use std::sync::Arc;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let log = Arc::new(AuditLog::new(tmp.path().join("audit.jsonl")).unwrap());

    let handler = AuditingApprovalHandler::new(Arc::new(AutoPendingHandler), Arc::clone(&log));
    let approval = make_approval_for_channel("cli", "");
    let status = handler
        .request_approval("async-step", "Agent output", &approval)
        .await;

    assert_eq!(status, ApprovalStatus::Pending);

    let entries = log.read_all().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[1].kind, AuditKind::ApprovalResolved);
    // Pending maps to "timed_out" in the audit record for compliance-field
    // stability. As of #419, Pending maps to WorkflowError::ApprovalPending
    // (not ApprovalTimedOut as before). The audit record still writes "timed_out"
    // so compliance consumers are not disrupted; the actual handler status is
    // preserved in the separate "original_status" field (see assertion below).
    assert_eq!(
        entries[1].metadata["decision"], "timed_out",
        "pending decision must be recorded as timed_out in metadata"
    );
    // When the handler returns Pending (not TimedOut), the audit record must
    // also include "original_status": "pending" so operators can distinguish
    // a genuine timeout from a handler that returned Pending (e.g. mis-configured
    // async handler). The compliance-facing "decision" field is stable.
    assert_eq!(
        entries[1].metadata["original_status"], "pending",
        "Pending must set original_status to 'pending' in metadata"
    );
}

#[tokio::test]
async fn auditing_handler_populates_workflow_and_agent_context() {
    use crate::runtime::audit::AuditLog;
    use std::sync::Arc;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let log = Arc::new(AuditLog::new(tmp.path().join("audit.jsonl")).unwrap());

    let handler = AuditingApprovalHandler::new(Arc::new(AutoApproveHandler), Arc::clone(&log))
        .with_workflow("deploy-workflow")
        .with_agent("deploy-bot");
    let approval = make_approval_for_channel("cli", "");
    handler
        .request_approval("release", "output", &approval)
        .await;

    let entries = log.read_all().unwrap();
    assert_eq!(entries[0].workflow.as_deref(), Some("deploy-workflow"));
    assert_eq!(entries[0].agent.as_deref(), Some("deploy-bot"));
    assert_eq!(entries[1].workflow.as_deref(), Some("deploy-workflow"));
    assert_eq!(entries[1].agent.as_deref(), Some("deploy-bot"));
}
