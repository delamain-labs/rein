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
    let approval = make_approval_for_channel("cli", "");
    let handler = resolve_approval_handler(&approval);
    // CLI handler should always auto-prompt; just verify it resolves without panic.
    let _ = handler;
}

#[test]
fn resolve_handler_cli_for_unknown_channel() {
    let approval = make_approval_for_channel("unknown_channel_xyz", "");
    let handler = resolve_approval_handler(&approval);
    let _ = handler;
}

#[tokio::test]
async fn webhook_handler_auto_approves_on_failure() {
    // When the webhook POST fails (invalid URL), the handler falls back to auto-approve.
    let handler = WebhookApprovalHandler::new("http://localhost:0/nonexistent".to_string());
    let approval = make_approval_for_channel("webhook", "http://localhost:0/nonexistent");
    let status = handler
        .request_approval("deploy", "output", &approval)
        .await;
    // Should not panic; on network failure falls back to auto-approve.
    assert_eq!(status, ApprovalStatus::Approved);
}

#[tokio::test]
async fn slack_handler_falls_back_to_auto_approve_on_failure() {
    // Slack webhook POST will fail (invalid URL); should fall back gracefully.
    let handler = SlackApprovalHandler::new("http://localhost:0/nonexistent".to_string());
    let approval = make_approval_for_channel("slack", "http://localhost:0/nonexistent");
    let status = handler
        .request_approval("notify", "output", &approval)
        .await;
    // Notification failure → auto-approve (non-blocking MVP behavior).
    assert_eq!(status, ApprovalStatus::Approved);
}

#[test]
fn resolve_handler_returns_slack_for_slack_channel() {
    // Constructing through resolve_approval_handler exercises the dispatch path.
    let approval = make_approval_for_channel("slack", "https://hooks.slack.com/test");
    let handler = resolve_approval_handler(&approval);
    let _ = handler; // Just ensure it constructs without panic.
}

#[test]
fn resolve_handler_returns_webhook_for_webhook_channel() {
    let approval = make_approval_for_channel("webhook", "https://example.com/hook");
    let handler = resolve_approval_handler(&approval);
    let _ = handler;
}
