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
