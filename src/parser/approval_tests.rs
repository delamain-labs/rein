use crate::ast::{ApprovalKind, CollaborationMode};
use crate::parser::parse;

#[test]
fn approve_with_timeout() {
    let f = parse(
        r##"
        agent a { model: "gpt-4o" }
        workflow w {
            trigger: event
            step review {
                agent: a
                approve: human via slack("#approvals") timeout "4h"
            }
        }
    "##,
    )
    .unwrap();
    let step = &f.workflows[0].steps[0];
    let approval = step.approval.as_ref().unwrap();
    assert_eq!(approval.kind, ApprovalKind::Approve);
    assert_eq!(approval.channel, "slack");
    assert_eq!(approval.destination, "#approvals");
    assert_eq!(approval.timeout.as_deref(), Some("4h"));
}

#[test]
fn collaborate_with_mode() {
    let f = parse(
        r#"
        agent a { model: "gpt-4o" }
        workflow w {
            trigger: event
            step edit_draft {
                agent: a
                collaborate: human via dashboard("editor") {
                    mode: edit
                }
            }
        }
    "#,
    )
    .unwrap();
    let step = &f.workflows[0].steps[0];
    let approval = step.approval.as_ref().unwrap();
    assert_eq!(approval.kind, ApprovalKind::Collaborate);
    assert_eq!(approval.channel, "dashboard");
    assert_eq!(approval.mode, Some(CollaborationMode::Edit));
}

#[test]
fn collaborate_suggest_mode() {
    let f = parse(
        r#"
        agent a { model: "gpt-4o" }
        workflow w {
            trigger: event
            step s {
                agent: a
                collaborate: human via dashboard("reviewer") {
                    mode: suggest
                    timeout: "2h"
                }
            }
        }
    "#,
    )
    .unwrap();
    let step = &f.workflows[0].steps[0];
    let approval = step.approval.as_ref().unwrap();
    assert_eq!(approval.mode, Some(CollaborationMode::Suggest));
    assert_eq!(approval.timeout.as_deref(), Some("2h"));
}
