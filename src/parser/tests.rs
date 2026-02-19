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
        f.agents[0].model.as_ref().and_then(|v| v.as_literal()),
        Some("anthropic/claude-3-sonnet")
    );
}

#[test]
fn parse_model_string_literal_with_dashes() {
    let f = parse_ok(r#"agent foo { model: "gpt-4o" }"#);
    assert_eq!(f.agents[0].model.as_ref().and_then(|v| v.as_literal()), Some("gpt-4o"));
}

#[test]
fn parse_model_ident_still_works() {
    // Bare identifier must continue to work alongside string literals.
    let f = parse_ok("agent foo { model: anthropic }");
    assert_eq!(f.agents[0].model.as_ref().and_then(|v| v.as_literal()), Some("anthropic"));
}

#[test]
fn error_model_invalid_value() {
    // A dollar amount is neither an ident nor a string — must error.
    let err = parse_err("agent foo { model: $5 }");
    assert!(err.message.contains("expected value"), "got: {}", err.message);
}

// ── env() function parsing ────────────────────────────────────────────────

#[test]
fn parse_env_in_model_field() {
    let f = parse_ok(r#"agent foo { model: env("MODEL_NAME") }"#);
    match &f.agents[0].model {
        Some(crate::ast::ValueExpr::EnvRef { var_name, .. }) => {
            assert_eq!(var_name, "MODEL_NAME");
        }
        other => panic!("expected EnvRef, got: {other:?}"),
    }
}

#[test]
fn env_missing_lparen_errors() {
    let err = parse_err(r#"agent foo { model: env "KEY" }"#);
    assert!(err.message.contains("("), "got: {}", err.message);
}

#[test]
fn env_missing_string_arg_errors() {
    // env() requires a string literal, not a bare identifier
    let err = parse_err("agent foo { model: env(KEY) }");
    assert!(
        err.message.contains("string argument"),
        "got: {}",
        err.message
    );
}

#[test]
fn env_missing_rparen_errors() {
    let err = parse_err(r#"agent foo { model: env("KEY" }"#);
    assert!(err.message.contains(")"), "got: {}", err.message);
}

#[test]
fn env_resolve_with_present_var() {
    let expr = crate::ast::ValueExpr::EnvRef {
        var_name: "MY_KEY".to_string(),
        span: crate::ast::Span::new(0, 1),
    };
    let lookup = |name: &str| {
        if name == "MY_KEY" { Some("secret_value".to_string()) } else { None }
    };
    assert_eq!(expr.resolve_with(lookup).unwrap(), "secret_value");
}

#[test]
fn env_resolve_with_missing_var() {
    let expr = crate::ast::ValueExpr::EnvRef {
        var_name: "MISSING_KEY".to_string(),
        span: crate::ast::Span::new(0, 1),
    };
    let lookup = |_: &str| None;
    let err = expr.resolve_with(lookup).unwrap_err();
    assert!(
        matches!(err, crate::ast::ResolveError::EnvVarNotSet(ref name) if name == "MISSING_KEY"),
        "got: {err}"
    );
}

#[test]
fn literal_value_resolves_directly() {
    let expr = crate::ast::ValueExpr::Literal("openai".to_string());
    let lookup = |_: &str| None; // shouldn't be called
    assert_eq!(expr.resolve_with(lookup).unwrap(), "openai");
}

// ── Minimal agent ─────────────────────────────────────────────────────────

#[test]
fn parse_minimal_agent() {
    let f = parse_ok("agent foo { model: anthropic }");
    assert_eq!(f.agents.len(), 1);
    let a = &f.agents[0];
    assert_eq!(a.name, "foo");
    assert_eq!(a.model.as_ref().and_then(|v| v.as_literal()), Some("anthropic"));
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
    assert_eq!(a.model.as_ref().and_then(|v| v.as_literal()), Some("anthropic"));
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
    assert_eq!(f.agents[0].model.as_ref().and_then(|v| v.as_literal()), Some("anthropic"));
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

// ── Workflow parsing ──────────────────────────────────────────────────────

#[test]
fn parse_simple_workflow() {
    let file = parse_ok(r#"
        agent triage { model: openai can [ zendesk.read_ticket ] }
        workflow pipeline {
            trigger: incoming_ticket
            stages: [triage]
        }
    "#);
    assert_eq!(file.workflows.len(), 1);
    let wf = &file.workflows[0];
    assert_eq!(wf.name, "pipeline");
    assert_eq!(wf.trigger, "incoming_ticket");
    assert_eq!(wf.stages.len(), 1);
    assert_eq!(wf.stages[0].agent, "triage");
}

#[test]
fn parse_workflow_multiple_stages() {
    let file = parse_ok(r#"
        agent a { model: openai }
        agent b { model: openai }
        agent c { model: openai }
        workflow pipe {
            trigger: event
            stages: [a, b, c]
        }
    "#);
    assert_eq!(file.workflows[0].stages.len(), 3);
    assert_eq!(file.workflows[0].stages[0].agent, "a");
    assert_eq!(file.workflows[0].stages[1].agent, "b");
    assert_eq!(file.workflows[0].stages[2].agent, "c");
}

#[test]
fn parse_workflow_stages_without_commas() {
    let file = parse_ok(r#"
        agent a { model: openai }
        agent b { model: openai }
        workflow pipe {
            trigger: event
            stages: [a b]
        }
    "#);
    assert_eq!(file.workflows[0].stages.len(), 2);
}

#[test]
fn parse_workflow_missing_trigger_errors() {
    let err = parse_err(r#"
        workflow pipe {
            stages: [a]
        }
    "#);
    assert!(err.message.contains("trigger"), "err: {}", err.message);
}

#[test]
fn parse_workflow_empty_stages_errors() {
    let err = parse_err(r#"
        workflow pipe {
            trigger: event
            stages: []
        }
    "#);
    assert!(err.message.contains("at least one stage"), "err: {}", err.message);
}

#[test]
fn parse_workflow_duplicate_trigger_errors() {
    let err = parse_err(r#"
        workflow pipe {
            trigger: a
            trigger: b
            stages: [x]
        }
    "#);
    assert!(err.message.contains("duplicate"), "err: {}", err.message);
}

#[test]
fn parse_file_with_agents_and_workflows() {
    let file = parse_ok(r#"
        agent triage { model: openai can [ zendesk.read_ticket ] }
        agent responder { model: anthropic can [ zendesk.reply_ticket ] }
        workflow pipeline {
            trigger: ticket
            stages: [triage, responder]
        }
    "#);
    assert_eq!(file.agents.len(), 2);
    assert_eq!(file.workflows.len(), 1);
}

#[test]
fn parse_multiple_workflows() {
    let file = parse_ok(r#"
        agent a { model: openai }
        workflow w1 { trigger: e1 stages: [a] }
        workflow w2 { trigger: e2 stages: [a] }
    "#);
    assert_eq!(file.workflows.len(), 2);
    assert_eq!(file.workflows[0].name, "w1");
    assert_eq!(file.workflows[1].name, "w2");
}

// ── Provider block tests ──────────────────────────────────────────────────

#[test]
fn parse_provider_basic() {
    let f = parse_ok(r#"provider anthropic { model: "claude-haiku" key: env("ANTHROPIC_KEY") }"#);
    assert_eq!(f.providers.len(), 1);
    assert_eq!(f.providers[0].name, "anthropic");
    assert_eq!(
        f.providers[0].model.as_ref().and_then(|v| v.as_literal()),
        Some("claude-haiku")
    );
    match &f.providers[0].key {
        Some(crate::ast::ValueExpr::EnvRef { var_name, .. }) => {
            assert_eq!(var_name, "ANTHROPIC_KEY");
        }
        other => panic!("expected EnvRef, got: {other:?}"),
    }
}

#[test]
fn parse_provider_model_only() {
    let f = parse_ok(r#"provider openai { model: "gpt-4o" }"#);
    assert_eq!(f.providers[0].name, "openai");
    assert!(f.providers[0].key.is_none());
}

#[test]
fn parse_multiple_providers() {
    let src = r#"
        provider anthropic { model: "claude-haiku" key: env("A_KEY") }
        provider openai { model: "gpt-4o" key: env("O_KEY") }
        agent test { model: openai }
    "#;
    let f = parse_ok(src);
    assert_eq!(f.providers.len(), 2);
    assert_eq!(f.agents.len(), 1);
}

#[test]
fn parse_provider_duplicate_model_errors() {
    let err = parse_err("provider x { model: a model: b }");
    assert!(err.message.contains("duplicate"), "got: {}", err.message);
}

#[test]
fn parse_provider_duplicate_key_errors() {
    let err = parse_err(r#"provider x { key: env("A") key: env("B") }"#);
    assert!(err.message.contains("duplicate"), "got: {}", err.message);
}

#[test]
fn parse_provider_unknown_field_errors() {
    let err = parse_err("provider x { model: a budget: $5 }");
    assert!(err.message.contains("unexpected"), "got: {}", err.message);
}

// ── Step block tests ──────────────────────────────────────────────────────

#[test]
fn parse_workflow_with_step_blocks() {
    let src = r#"
        agent triage { model: openai }
        workflow support {
            trigger: ticket
            step classify {
                agent: triage
                goal: "Classify this ticket"
            }
        }
    "#;
    let f = parse_ok(src);
    assert_eq!(f.workflows[0].steps.len(), 1);
    assert_eq!(f.workflows[0].steps[0].name, "classify");
    assert_eq!(f.workflows[0].steps[0].agent, "triage");
    assert_eq!(f.workflows[0].steps[0].goal.as_deref(), Some("Classify this ticket"));
}

#[test]
fn parse_workflow_with_multiple_steps() {
    let src = r#"
        agent a { model: openai }
        agent b { model: openai }
        workflow pipe {
            trigger: event
            step first {
                agent: a
                goal: "Do step one"
            }
            step second {
                agent: b
                goal: "Do step two"
            }
        }
    "#;
    let f = parse_ok(src);
    assert_eq!(f.workflows[0].steps.len(), 2);
    assert_eq!(f.workflows[0].steps[0].name, "first");
    assert_eq!(f.workflows[0].steps[1].name, "second");
}

#[test]
fn parse_step_without_goal() {
    let src = r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            step s1 {
                agent: a
            }
        }
    "#;
    let f = parse_ok(src);
    assert!(f.workflows[0].steps[0].goal.is_none());
}

#[test]
fn parse_step_missing_agent_errors() {
    let err = parse_err(r#"
        workflow w {
            trigger: event
            step s1 {
                goal: "Do something"
            }
        }
    "#);
    assert!(err.message.contains("missing"), "got: {}", err.message);
}

#[test]
fn parse_step_duplicate_agent_errors() {
    let err = parse_err(r#"
        workflow w {
            trigger: event
            step s1 {
                agent: a
                agent: b
            }
        }
    "#);
    assert!(err.message.contains("duplicate"), "got: {}", err.message);
}

#[test]
fn parse_workflow_mixed_stages_and_steps() {
    let src = r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            stages: [a]
            step extra {
                agent: a
                goal: "Extra processing"
            }
        }
    "#;
    let f = parse_ok(src);
    assert_eq!(f.workflows[0].stages.len(), 1);
    assert_eq!(f.workflows[0].steps.len(), 1);
}
