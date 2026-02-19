use super::*;
use crate::ast::Constraint;

fn parse_ok(src: &str) -> ReinFile {
    parse(src).expect("expected parse to succeed")
}

fn parse_err(src: &str) -> ParseError {
    parse(src).expect_err("expected parse to fail")
}

// ── String literal model values ───────────────────────────────────────────

#[test]
fn parse_model_as_string_literal() {
    let f = parse_ok(r#"agent foo { model: "anthropic/claude-3-sonnet" }"#);
    assert_eq!(
        f.agents[0].model.as_deref(),
        Some("anthropic/claude-3-sonnet")
    );
}

#[test]
fn parse_model_string_literal_with_dashes() {
    let f = parse_ok(r#"agent foo { model: "gpt-4o" }"#);
    assert_eq!(f.agents[0].model.as_deref(), Some("gpt-4o"));
}

#[test]
fn parse_model_ident_still_works() {
    // Bare identifier must continue to work alongside string literals.
    let f = parse_ok("agent foo { model: anthropic }");
    assert_eq!(f.agents[0].model.as_deref(), Some("anthropic"));
}

#[test]
fn error_model_invalid_value() {
    // A dollar amount is neither an ident nor a string — must error.
    let err = parse_err("agent foo { model: $5 }");
    assert!(err.message.contains("model name"), "got: {}", err.message);
}

// ── Minimal agent ─────────────────────────────────────────────────────────

#[test]
fn parse_minimal_agent() {
    let f = parse_ok("agent foo { model: anthropic }");
    assert_eq!(f.agents.len(), 1);
    let a = &f.agents[0];
    assert_eq!(a.name, "foo");
    assert_eq!(a.model.as_deref(), Some("anthropic"));
    assert!(a.can.is_empty());
    assert!(a.cannot.is_empty());
    assert!(a.budget.is_none());
}

#[test]
fn parse_agent_no_model() {
    let f = parse_ok("agent bot { }");
    assert_eq!(f.agents[0].model, None);
}

// ── Capabilities ──────────────────────────────────────────────────────────

#[test]
fn parse_can_list() {
    let src = r#"
agent foo {
can [
    zendesk.read_ticket
    zendesk.reply_ticket
]
}"#;
    let f = parse_ok(src);
    let a = &f.agents[0];
    assert_eq!(a.can.len(), 2);
    assert_eq!(a.can[0].namespace, "zendesk");
    assert_eq!(a.can[0].action, "read_ticket");
    assert_eq!(a.can[1].action, "reply_ticket");
}

#[test]
fn parse_cannot_list() {
    let src = "agent foo { cannot [ zendesk.delete_ticket ] }";
    let f = parse_ok(src);
    assert_eq!(f.agents[0].cannot[0].action, "delete_ticket");
}

#[test]
fn parse_up_to_constraint() {
    let src = "agent foo { can [ zendesk.refund up to $50 ] }";
    let f = parse_ok(src);
    let cap = &f.agents[0].can[0];
    assert_eq!(cap.action, "refund");
    match &cap.constraint {
        Some(Constraint::MonetaryCap { amount, currency }) => {
            assert_eq!(*amount, 5000u64);
            assert_eq!(currency, "USD");
        }
        None => panic!("expected MonetaryCap constraint"),
    }
}

// ── Budget ────────────────────────────────────────────────────────────────

#[test]
fn parse_budget() {
    let src = "agent foo { budget: $0.03 per ticket }";
    let f = parse_ok(src);
    let b = f.agents[0].budget.as_ref().unwrap();
    assert_eq!(b.amount, 3u64);
    assert_eq!(b.currency, "USD");
    assert_eq!(b.unit, "ticket");
}

// ── Full agent ────────────────────────────────────────────────────────────

#[test]
fn parse_full_agent() {
    let src = r#"
agent support_triage {
model: anthropic

can [
    zendesk.read_ticket
    zendesk.reply_ticket
    zendesk.refund up to $50
]

cannot [
    zendesk.delete_ticket
    zendesk.admin
]

budget: $0.03 per ticket
}"#;
    let f = parse_ok(src);
    let a = &f.agents[0];
    assert_eq!(a.name, "support_triage");
    assert_eq!(a.model.as_deref(), Some("anthropic"));
    assert_eq!(a.can.len(), 3);
    assert_eq!(a.cannot.len(), 2);
    assert!(a.budget.is_some());
    // constraint on refund
    assert!(a.can[2].constraint.is_some());
}

// ── Multiple agents ───────────────────────────────────────────────────────

#[test]
fn parse_multiple_agents() {
    let src = r#"
agent alpha { model: openai }
agent beta  { model: anthropic }
"#;
    let f = parse_ok(src);
    assert_eq!(f.agents.len(), 2);
    assert_eq!(f.agents[0].name, "alpha");
    assert_eq!(f.agents[1].name, "beta");
}

// ── Comments ──────────────────────────────────────────────────────────────

#[test]
fn parse_with_comments() {
    let src = r#"
// top-level comment
agent foo {
// model comment
model: anthropic /* inline */
}
"#;
    let f = parse_ok(src);
    assert_eq!(f.agents[0].model.as_deref(), Some("anthropic"));
}

// ── Span accuracy ─────────────────────────────────────────────────────────

#[test]
fn capability_span_simple() {
    let src = "agent foo { can [ zendesk.read_ticket ] }";
    let f = parse_ok(src);
    let cap = &f.agents[0].can[0];
    let text = &src[cap.span.start..cap.span.end];
    assert_eq!(text, "zendesk.read_ticket");
}

#[test]
fn capability_span_with_constraint() {
    let src = "agent foo { can [ zendesk.refund up to $50 ] }";
    let f = parse_ok(src);
    let cap = &f.agents[0].can[0];
    let text = &src[cap.span.start..cap.span.end];
    assert_eq!(text, "zendesk.refund up to $50");
}

// ── Error paths ───────────────────────────────────────────────────────────

#[test]
fn error_missing_agent_name() {
    let err = parse_err("agent { }");
    assert!(err.message.contains("identifier"), "got: {}", err.message);
}

#[test]
fn error_missing_lbrace() {
    let err = parse_err("agent foo }");
    assert!(
        err.message.contains("LBrace") || err.message.contains('{'),
        "got: {}",
        err.message
    );
}

#[test]
fn error_missing_rbrace() {
    let err = parse_err("agent foo {");
    assert!(
        err.message.contains("end of file") || err.message.contains('}'),
        "got: {}",
        err.message
    );
}

#[test]
fn error_can_without_bracket() {
    let err = parse_err("agent foo { can zendesk.read_ticket }");
    assert!(
        err.message.contains("LBracket") || err.message.contains('['),
        "got: {}",
        err.message
    );
}

#[test]
fn error_duplicate_model() {
    let err = parse_err("agent foo { model: a model: b }");
    assert!(
        err.message.contains("duplicate field 'model'"),
        "got: {}",
        err.message
    );
}

#[test]
fn error_duplicate_can() {
    let err = parse_err("agent foo { can [ zendesk.read ] can [ zendesk.write ] }");
    assert!(
        err.message.contains("duplicate field 'can'"),
        "got: {}",
        err.message
    );
}

#[test]
fn error_duplicate_cannot() {
    let err = parse_err("agent foo { cannot [ zendesk.read ] cannot [ zendesk.write ] }");
    assert!(
        err.message.contains("duplicate field 'cannot'"),
        "got: {}",
        err.message
    );
}

#[test]
fn error_duplicate_budget() {
    let err = parse_err("agent foo { budget: $0.03 per ticket budget: $0.05 per ticket }");
    assert!(
        err.message.contains("duplicate field 'budget'"),
        "got: {}",
        err.message
    );
}

#[test]
fn error_budget_missing_dollar() {
    let err = parse_err("agent foo { budget: notadollar per ticket }");
    assert!(
        err.message.to_lowercase().contains("dollar") || err.message.contains('$'),
        "got: {}",
        err.message
    );
}
