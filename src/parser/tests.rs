use super::*;
use crate::ast::{Constraint, TypeExpr};

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
    assert_eq!(
        f.agents[0].model.as_ref().and_then(|v| v.as_literal()),
        Some("gpt-4o")
    );
}

#[test]
fn parse_model_ident_still_works() {
    // Bare identifier must continue to work alongside string literals.
    let f = parse_ok("agent foo { model: anthropic }");
    assert_eq!(
        f.agents[0].model.as_ref().and_then(|v| v.as_literal()),
        Some("anthropic")
    );
}

#[test]
fn error_model_invalid_value() {
    // A dollar amount is neither an ident nor a string — must error.
    let err = parse_err("agent foo { model: $5 }");
    assert!(
        err.message.contains("expected value"),
        "got: {}",
        err.message
    );
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
        default: None,
        span: crate::ast::Span::new(0, 1),
    };
    let lookup = |name: &str| {
        if name == "MY_KEY" {
            Some("secret_value".to_string())
        } else {
            None
        }
    };
    assert_eq!(expr.resolve_with(lookup).unwrap(), "secret_value");
}

#[test]
fn env_resolve_with_missing_var() {
    let expr = crate::ast::ValueExpr::EnvRef {
        var_name: "MISSING_KEY".to_string(),
        default: None,
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
    assert_eq!(
        a.model.as_ref().and_then(|v| v.as_literal()),
        Some("anthropic")
    );
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
    assert_eq!(
        a.model.as_ref().and_then(|v| v.as_literal()),
        Some("anthropic")
    );
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
    assert_eq!(
        f.agents[0].model.as_ref().and_then(|v| v.as_literal()),
        Some("anthropic")
    );
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
fn parse_budget_euro() {
    let f = parse_ok("agent bot { model: openai budget: €0.05 per request }");
    let b = f.agents[0].budget.as_ref().unwrap();
    assert_eq!(b.amount, 5);
    assert_eq!(b.currency, "EUR");
}

#[test]
fn parse_budget_pound() {
    let f = parse_ok("agent bot { model: openai budget: £1.00 per ticket }");
    let b = f.agents[0].budget.as_ref().unwrap();
    assert_eq!(b.amount, 100);
    assert_eq!(b.currency, "GBP");
}

#[test]
fn parse_budget_yen() {
    let f = parse_ok("agent bot { model: openai budget: ¥500 per run }");
    let b = f.agents[0].budget.as_ref().unwrap();
    assert_eq!(b.amount, 50000);
    assert_eq!(b.currency, "JPY");
}

#[test]
fn parse_capability_constraint_euro() {
    let f = parse_ok("agent bot { model: openai can [stripe.refund up to €100] }");
    let cap = &f.agents[0].can[0];
    if let Some(crate::ast::Constraint::MonetaryCap { amount, currency }) = &cap.constraint {
        assert_eq!(*amount, 10000);
        assert_eq!(currency, "EUR");
    } else {
        panic!("expected MonetaryCap");
    }
}

#[test]
fn error_budget_missing_currency() {
    let err = parse_err("agent foo { budget: notadollar per ticket }");
    assert!(
        err.message.contains("currency") || err.message.contains('$'),
        "got: {}",
        err.message
    );
}

// ── Workflow parsing ──────────────────────────────────────────────────────

#[test]
fn parse_simple_workflow() {
    let file = parse_ok(
        r#"
        agent triage { model: openai can [ zendesk.read_ticket ] }
        workflow pipeline {
            trigger: incoming_ticket
            stages: [triage]
        }
    "#,
    );
    assert_eq!(file.workflows.len(), 1);
    let wf = &file.workflows[0];
    assert_eq!(wf.name, "pipeline");
    assert_eq!(wf.trigger, "incoming_ticket");
    assert_eq!(wf.stages.len(), 1);
    assert_eq!(wf.stages[0].agent, "triage");
}

#[test]
fn parse_workflow_multiple_stages() {
    let file = parse_ok(
        r#"
        agent a { model: openai }
        agent b { model: openai }
        agent c { model: openai }
        workflow pipe {
            trigger: event
            stages: [a, b, c]
        }
    "#,
    );
    assert_eq!(file.workflows[0].stages.len(), 3);
    assert_eq!(file.workflows[0].stages[0].agent, "a");
    assert_eq!(file.workflows[0].stages[1].agent, "b");
    assert_eq!(file.workflows[0].stages[2].agent, "c");
}

#[test]
fn parse_workflow_stages_without_commas() {
    let file = parse_ok(
        r#"
        agent a { model: openai }
        agent b { model: openai }
        workflow pipe {
            trigger: event
            stages: [a b]
        }
    "#,
    );
    assert_eq!(file.workflows[0].stages.len(), 2);
}

#[test]
fn parse_workflow_trigger_multi_word() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow pipe {
            trigger: new ticket in zendesk
            stages: [a]
        }
    "#,
    );
    assert_eq!(f.workflows[0].trigger, "new ticket in zendesk");
}

#[test]
fn parse_workflow_trigger_string_literal() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow pipe {
            trigger: "new message in channel"
            stages: [a]
        }
    "#,
    );
    assert_eq!(f.workflows[0].trigger, "new message in channel");
}

#[test]
fn parse_workflow_trigger_single_word() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow pipe {
            trigger: event
            stages: [a]
        }
    "#,
    );
    assert_eq!(f.workflows[0].trigger, "event");
}

#[test]
fn parse_workflow_missing_trigger_errors() {
    let err = parse_err(
        r#"
        workflow pipe {
            stages: [a]
        }
    "#,
    );
    assert!(err.message.contains("trigger"), "err: {}", err.message);
}

#[test]
fn parse_workflow_empty_stages_errors() {
    let err = parse_err(
        r#"
        workflow pipe {
            trigger: event
            stages: []
        }
    "#,
    );
    assert!(
        err.message.contains("at least one stage"),
        "err: {}",
        err.message
    );
}

#[test]
fn parse_workflow_duplicate_trigger_errors() {
    let err = parse_err(
        r#"
        workflow pipe {
            trigger: a
            trigger: b
            stages: [x]
        }
    "#,
    );
    assert!(err.message.contains("duplicate"), "err: {}", err.message);
}

#[test]
fn parse_file_with_agents_and_workflows() {
    let file = parse_ok(
        r#"
        agent triage { model: openai can [ zendesk.read_ticket ] }
        agent responder { model: anthropic can [ zendesk.reply_ticket ] }
        workflow pipeline {
            trigger: ticket
            stages: [triage, responder]
        }
    "#,
    );
    assert_eq!(file.agents.len(), 2);
    assert_eq!(file.workflows.len(), 1);
}

#[test]
fn parse_multiple_workflows() {
    let file = parse_ok(
        r#"
        agent a { model: openai }
        workflow w1 { trigger: e1 stages: [a] }
        workflow w2 { trigger: e2 stages: [a] }
    "#,
    );
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
    assert_eq!(
        f.workflows[0].steps[0].goal.as_deref(),
        Some("Classify this ticket")
    );
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
    let err = parse_err(
        r#"
        workflow w {
            trigger: event
            step s1 {
                goal: "Do something"
            }
        }
    "#,
    );
    assert!(err.message.contains("missing"), "got: {}", err.message);
}

#[test]
fn parse_step_duplicate_agent_errors() {
    let err = parse_err(
        r#"
        workflow w {
            trigger: event
            step s1 {
                agent: a
                agent: b
            }
        }
    "#,
    );
    assert!(err.message.contains("duplicate"), "got: {}", err.message);
}

#[test]
fn parse_step_duplicate_goal_errors() {
    let err = parse_err(
        r#"
        workflow w {
            trigger: event
            step s1 {
                agent: a
                goal: "first"
                goal: "second"
            }
        }
    "#,
    );
    assert!(err.message.contains("duplicate"), "got: {}", err.message);
}

#[test]
fn parse_step_goal_env_ref_errors() {
    let err = parse_err(
        r#"
        workflow w {
            trigger: event
            step s1 {
                agent: a
                goal: env("SECRET")
            }
        }
    "#,
    );
    assert!(
        err.message.contains("goal must be a string literal"),
        "got: {}",
        err.message
    );
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

// ───── Defaults block tests ─────

#[test]
fn parse_defaults_with_model() {
    let f = parse_ok("defaults { model: anthropic }");
    let d = f.defaults.unwrap();
    assert!(d.model.is_some());
    assert!(d.budget.is_none());
}

#[test]
fn parse_defaults_with_budget() {
    let f = parse_ok("defaults { budget: $0.05 per run }");
    let d = f.defaults.unwrap();
    assert!(d.budget.is_some());
    assert_eq!(d.budget.unwrap().amount, 5);
}

#[test]
fn parse_defaults_with_all_fields() {
    let f = parse_ok("defaults { model: anthropic budget: $1.00 per request }");
    let d = f.defaults.unwrap();
    assert!(d.model.is_some());
    assert!(d.budget.is_some());
}

#[test]
fn parse_defaults_empty() {
    let f = parse_ok("defaults {}");
    let d = f.defaults.unwrap();
    assert!(d.model.is_none());
    assert!(d.budget.is_none());
}

#[test]
fn parse_defaults_before_agents() {
    let f = parse_ok(
        r#"
        defaults { model: openai }
        agent bot { can [zendesk.read] }
    "#,
    );
    assert!(f.defaults.is_some());
    assert_eq!(f.agents.len(), 1);
}

#[test]
fn parse_duplicate_defaults_errors() {
    let err = parse_err("defaults {} defaults {}");
    assert!(err.message.contains("duplicate"), "got: {}", err.message);
}

#[test]
fn parse_defaults_duplicate_model_errors() {
    let err = parse_err("defaults { model: a model: b }");
    assert!(err.message.contains("duplicate"), "got: {}", err.message);
}

#[test]
fn parse_defaults_unknown_field_errors() {
    let err = parse_err("defaults { foo: bar }");
    assert!(err.message.contains("unexpected"), "got: {}", err.message);
}

#[test]
fn parse_no_defaults() {
    let f = parse_ok("agent bot { model: openai }");
    assert!(f.defaults.is_none());
}

// ───── Guardrails tests ─────

#[test]
fn parse_agent_with_guardrails() {
    let f = parse_ok(
        r#"agent bot {
            model: openai
            guardrails {
                output_filter {
                    pii_detection: redact
                    toxicity: block
                }
            }
        }"#,
    );
    let g = f.agents[0].guardrails.as_ref().unwrap();
    assert_eq!(g.sections.len(), 1);
    assert_eq!(g.sections[0].name, "output_filter");
    assert_eq!(g.sections[0].rules.len(), 2);
    assert_eq!(g.sections[0].rules[0].key, "pii_detection");
    assert_eq!(g.sections[0].rules[0].value, "redact");
    assert_eq!(g.sections[0].rules[1].key, "toxicity");
    assert_eq!(g.sections[0].rules[1].value, "block");
}

#[test]
fn parse_agent_guardrails_multiple_sections() {
    let f = parse_ok(
        r#"agent bot {
            model: openai
            guardrails {
                output_filter {
                    pii_detection: redact
                }
                escalation {
                    low_confidence: escalate
                }
            }
        }"#,
    );
    let g = f.agents[0].guardrails.as_ref().unwrap();
    assert_eq!(g.sections.len(), 2);
    assert_eq!(g.sections[0].name, "output_filter");
    assert_eq!(g.sections[1].name, "escalation");
}

#[test]
fn parse_agent_guardrails_empty() {
    let f = parse_ok(
        r#"agent bot {
            model: openai
            guardrails {}
        }"#,
    );
    let g = f.agents[0].guardrails.as_ref().unwrap();
    assert!(g.sections.is_empty());
}

#[test]
fn parse_agent_duplicate_guardrails_errors() {
    let err = parse_err(
        r#"agent bot {
            model: openai
            guardrails {}
            guardrails {}
        }"#,
    );
    assert!(err.message.contains("duplicate"), "got: {}", err.message);
}

#[test]
fn parse_guardrails_duplicate_section_errors() {
    let err = parse_err(
        r#"agent bot {
            model: openai
            guardrails {
                output_filter { pii: redact }
                output_filter { toxicity: block }
            }
        }"#,
    );
    assert!(err.message.contains("duplicate"), "got: {}", err.message);
}

#[test]
fn parse_agent_no_guardrails() {
    let f = parse_ok("agent bot { model: openai }");
    assert!(f.agents[0].guardrails.is_none());
}

// ───── Tool block tests ─────

#[test]
fn parse_tool_with_endpoint() {
    let f = parse_ok(r#"tool zendesk { endpoint: "https://api.zendesk.com/v2" }"#);
    assert_eq!(f.tools.len(), 1);
    assert_eq!(f.tools[0].name, "zendesk");
    assert!(f.tools[0].endpoint.is_some());
}

#[test]
fn parse_tool_with_all_fields() {
    let f = parse_ok(
        r#"tool zendesk {
            provider: rest_api
            endpoint: "https://api.zendesk.com/v2"
            key: env("ZENDESK_KEY")
        }"#,
    );
    assert_eq!(f.tools[0].name, "zendesk");
    assert!(f.tools[0].provider.is_some());
    assert!(f.tools[0].endpoint.is_some());
    assert!(f.tools[0].key.is_some());
}

#[test]
fn parse_tool_empty_block() {
    let f = parse_ok("tool empty_tool {}");
    assert_eq!(f.tools.len(), 1);
    assert_eq!(f.tools[0].name, "empty_tool");
    assert!(f.tools[0].endpoint.is_none());
}

#[test]
fn parse_tool_endpoint_env_ref() {
    let f = parse_ok(r#"tool api { endpoint: env("API_URL") }"#);
    assert!(f.tools[0].endpoint.is_some());
}

#[test]
fn parse_tool_duplicate_endpoint_errors() {
    let err = parse_err(
        r#"tool z {
            endpoint: "a"
            endpoint: "b"
        }"#,
    );
    assert!(err.message.contains("duplicate"), "got: {}", err.message);
}

#[test]
fn parse_tool_duplicate_provider_errors() {
    let err = parse_err(
        r#"tool z {
            provider: rest
            provider: mcp
        }"#,
    );
    assert!(err.message.contains("duplicate"), "got: {}", err.message);
}

#[test]
fn parse_tool_duplicate_key_errors() {
    let err = parse_err(
        r#"tool z {
            key: env("A")
            key: env("B")
        }"#,
    );
    assert!(err.message.contains("duplicate"), "got: {}", err.message);
}

#[test]
fn parse_tool_unknown_field_errors() {
    let err = parse_err(r#"tool z { foo: bar }"#);
    assert!(err.message.contains("unexpected"), "got: {}", err.message);
}

#[test]
fn parse_multiple_tools() {
    let f = parse_ok(
        r#"
        tool zendesk { endpoint: "https://zendesk.com" }
        tool shopify { endpoint: "https://shopify.com" }
    "#,
    );
    assert_eq!(f.tools.len(), 2);
}

// ── one of union type tests ─────────────────────────────────────────────

#[test]
fn parse_step_with_one_of_constraint() {
    let f = parse_ok(
        r#"
        agent classifier { model: openai }
        workflow support {
            trigger: ticket
            step classify {
                agent: classifier
                goal: "Classify the ticket"
                category: one of [billing, technical, general]
            }
        }
    "#,
    );
    let wf = &f.workflows[0];
    let step = &wf.steps[0];
    assert_eq!(step.output_constraints.len(), 1);
    let (name, type_expr) = &step.output_constraints[0];
    assert_eq!(name, "category");
    match type_expr {
        crate::ast::TypeExpr::OneOf { variants, .. } => {
            assert_eq!(variants, &["billing", "technical", "general"]);
        }
        other => panic!("expected OneOf, got {other:?}"),
    }
}

#[test]
fn parse_step_with_multiple_one_of_constraints() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            step s {
                agent: a
                category: one of [a, b]
                priority: one of [low, medium, high]
            }
        }
    "#,
    );
    let step = &f.workflows[0].steps[0];
    assert_eq!(step.output_constraints.len(), 2);
    assert_eq!(step.output_constraints[0].0, "category");
    assert_eq!(step.output_constraints[1].0, "priority");
}

#[test]
fn parse_one_of_empty_variants_errors() {
    let err = parse_err(
        r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            step s {
                agent: a
                category: one of []
            }
        }
    "#,
    );
    assert!(
        err.message.contains("at least one"),
        "got: {}",
        err.message
    );
}

#[test]
fn parse_one_of_trailing_comma() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            step s {
                agent: a
                status: one of [open, closed,]
            }
        }
    "#,
    );
    let step = &f.workflows[0].steps[0];
    let (_, type_expr) = &step.output_constraints[0];
    match type_expr {
        crate::ast::TypeExpr::OneOf { variants, .. } => {
            assert_eq!(variants, &["open", "closed"]);
        }
        other => panic!("expected OneOf, got {other:?}"),
    }
}

// ── type system tests ───────────────────────────────────────────────────

#[test]
fn parse_type_def_basic() {
    let f = parse_ok(
        r#"
        type TicketClassification {
            category: one of [billing, technical, general]
            confidence: percent
            tags: string[]
        }
    "#,
    );
    assert_eq!(f.types.len(), 1);
    let td = &f.types[0];
    assert_eq!(td.name, "TicketClassification");
    assert_eq!(td.fields.len(), 3);

    assert_eq!(td.fields[0].name, "category");
    assert!(matches!(&td.fields[0].type_expr, TypeExpr::OneOf { variants, .. } if variants.len() == 3));

    assert_eq!(td.fields[1].name, "confidence");
    assert!(matches!(&td.fields[1].type_expr, TypeExpr::Named { name, array: false } if name == "percent"));

    assert_eq!(td.fields[2].name, "tags");
    assert!(matches!(&td.fields[2].type_expr, TypeExpr::Named { name, array: true } if name == "string"));
}

#[test]
fn parse_type_def_builtins() {
    let f = parse_ok(
        r#"
        type Record {
            name: string
            age: int
            score: float
            active: bool
            price: currency
            elapsed: duration
        }
    "#,
    );
    let td = &f.types[0];
    assert_eq!(td.fields.len(), 6);
    let names: Vec<&str> = td.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, &["name", "age", "score", "active", "price", "elapsed"]);
}

#[test]
fn parse_type_def_with_range() {
    let f = parse_ok(
        r#"
        type Config {
            temperature: 0..100
        }
    "#,
    );
    let td = &f.types[0];
    assert!(matches!(&td.fields[0].type_expr, TypeExpr::Range { min, max } if min == "0" && max == "100"));
}

#[test]
fn parse_type_def_float_range() {
    let f = parse_ok(
        r#"
        type Config {
            temperature: 0.0..1.0
        }
    "#,
    );
    let td = &f.types[0];
    assert!(matches!(&td.fields[0].type_expr, TypeExpr::Range { min, max } if min == "0.0" && max == "1.0"));
}

#[test]
fn parse_multiple_type_defs() {
    let f = parse_ok(
        r#"
        type A { x: int }
        type B { y: string }
    "#,
    );
    assert_eq!(f.types.len(), 2);
    assert_eq!(f.types[0].name, "A");
    assert_eq!(f.types[1].name, "B");
}

#[test]
fn parse_type_with_agents() {
    let f = parse_ok(
        r#"
        type Output { status: one of [ok, error] }
        agent bot { model: openai }
    "#,
    );
    assert_eq!(f.types.len(), 1);
    assert_eq!(f.agents.len(), 1);
}

// ── import system tests ─────────────────────────────────────────────────

#[test]
fn parse_named_import() {
    let f = parse_ok(
        r#"
        import { classifier, responder } from "./agents/support.rein"
        agent classifier { model: openai }
        agent responder { model: openai }
    "#,
    );
    assert_eq!(f.imports.len(), 1);
    match &f.imports[0] {
        crate::ast::ImportDef::Named { names, source, .. } => {
            assert_eq!(names, &["classifier", "responder"]);
            assert_eq!(source, "./agents/support.rein");
        }
        other => panic!("expected Named import, got {other:?}"),
    }
}

#[test]
fn parse_glob_import() {
    let f = parse_ok(
        r#"
        import all from "./agents/"
    "#,
    );
    assert_eq!(f.imports.len(), 1);
    match &f.imports[0] {
        crate::ast::ImportDef::Glob { source, .. } => {
            assert_eq!(source, "./agents/");
        }
        other => panic!("expected Glob import, got {other:?}"),
    }
}

#[test]
fn parse_registry_import() {
    let f = parse_ok(
        r#"
        import from @rein/support
    "#,
    );
    assert_eq!(f.imports.len(), 1);
    match &f.imports[0] {
        crate::ast::ImportDef::Registry { scope, name, .. } => {
            assert_eq!(scope, "rein");
            assert_eq!(name, "support");
        }
        other => panic!("expected Registry import, got {other:?}"),
    }
}

#[test]
fn parse_multiple_imports() {
    let f = parse_ok(
        r#"
        import { bot } from "./bot.rein"
        import all from "./shared/"
        import from @rein/stdlib
        agent bot { model: openai }
    "#,
    );
    assert_eq!(f.imports.len(), 3);
    assert_eq!(f.agents.len(), 1);
}

#[test]
fn parse_import_single_name() {
    let f = parse_ok(
        r#"
        import { classifier } from "./classifier.rein"
    "#,
    );
    match &f.imports[0] {
        crate::ast::ImportDef::Named { names, .. } => {
            assert_eq!(names, &["classifier"]);
        }
        other => panic!("expected Named import, got {other:?}"),
    }
}

// ── route on block tests ────────────────────────────────────────────────

#[test]
fn parse_route_on_basic() {
    let f = parse_ok(
        r#"
        agent classifier { model: openai }
        agent billing_handler { model: openai }
        agent default_handler { model: openai }
        workflow support {
            trigger: ticket
            route on classify.category {
                billing -> step handle_billing {
                    agent: billing_handler
                    goal: "Handle billing"
                }
                _ -> step escalate {
                    agent: default_handler
                    goal: "Escalate"
                }
            }
        }
    "#,
    );
    let wf = &f.workflows[0];
    assert_eq!(wf.route_blocks.len(), 1);
    let rb = &wf.route_blocks[0];
    assert_eq!(rb.field_path, "classify.category");
    assert_eq!(rb.arms.len(), 2);
    assert!(matches!(&rb.arms[0].pattern, crate::ast::RoutePattern::Value(v) if v == "billing"));
    assert_eq!(rb.arms[0].step.name, "handle_billing");
    assert!(matches!(&rb.arms[1].pattern, crate::ast::RoutePattern::Wildcard));
    assert_eq!(rb.arms[1].step.name, "escalate");
}

#[test]
fn parse_route_on_multiple_arms() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        agent b { model: openai }
        agent c { model: openai }
        workflow w {
            trigger: event
            route on result.status {
                success -> step ok { agent: a }
                failure -> step fail { agent: b }
                _ -> step unknown { agent: c }
            }
        }
    "#,
    );
    let rb = &f.workflows[0].route_blocks[0];
    assert_eq!(rb.arms.len(), 3);
    assert!(matches!(&rb.arms[0].pattern, crate::ast::RoutePattern::Value(v) if v == "success"));
    assert!(matches!(&rb.arms[1].pattern, crate::ast::RoutePattern::Value(v) if v == "failure"));
    assert!(matches!(&rb.arms[2].pattern, crate::ast::RoutePattern::Wildcard));
}

#[test]
fn parse_route_on_with_steps() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        agent b { model: openai }
        workflow w {
            trigger: event
            step classify { agent: a goal: "Classify" }
            route on classify.type {
                billing -> step handle { agent: b }
            }
        }
    "#,
    );
    assert_eq!(f.workflows[0].steps.len(), 1);
    assert_eq!(f.workflows[0].route_blocks.len(), 1);
}

// ── parallel block tests ────────────────────────────────────────────────

#[test]
fn parse_parallel_block_basic() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        agent b { model: openai }
        workflow w {
            trigger: event
            parallel {
                step analyze { agent: a goal: "Analyze" }
                step summarize { agent: b goal: "Summarize" }
            }
        }
    "#,
    );
    assert_eq!(f.workflows[0].parallel_blocks.len(), 1);
    let pb = &f.workflows[0].parallel_blocks[0];
    assert_eq!(pb.steps.len(), 2);
    assert_eq!(pb.steps[0].name, "analyze");
    assert_eq!(pb.steps[1].name, "summarize");
}

#[test]
fn parse_parallel_with_sequential_steps() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        agent b { model: openai }
        agent c { model: openai }
        workflow w {
            trigger: event
            step first { agent: a }
            parallel {
                step p1 { agent: b }
                step p2 { agent: c }
            }
        }
    "#,
    );
    assert_eq!(f.workflows[0].steps.len(), 1);
    assert_eq!(f.workflows[0].parallel_blocks.len(), 1);
}

#[test]
fn parse_parallel_empty_errors() {
    let err = parse_err(
        r#"
        workflow w {
            trigger: event
            parallel {}
        }
    "#,
    );
    assert!(err.message.contains("at least one"), "got: {}", err.message);
}

// ── when expression tests ───────────────────────────────────────────────

#[test]
fn parse_step_with_when_percent() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            step x {
                agent: a
                when: confidence < 70%
            }
        }
    "#,
    );
    let step = &f.workflows[0].steps[0];
    let when = step.when.as_ref().unwrap();
    match when {
        crate::ast::WhenExpr::Comparison(c) => {
            assert_eq!(c.field, "confidence");
            assert_eq!(c.op, crate::ast::CompareOp::Lt);
            assert!(matches!(&c.value, crate::ast::WhenValue::Percent(p) if p == "70"));
        }
        other => panic!("expected Comparison, got {other:?}"),
    }
}

#[test]
fn parse_step_with_when_currency() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            step x {
                agent: a
                when: refund > $50.00
            }
        }
    "#,
    );
    let step = &f.workflows[0].steps[0];
    let when = step.when.as_ref().unwrap();
    match when {
        crate::ast::WhenExpr::Comparison(c) => {
            assert_eq!(c.field, "refund");
            assert_eq!(c.op, crate::ast::CompareOp::Gt);
            assert!(matches!(&c.value, crate::ast::WhenValue::Currency { symbol: '$', amount: 5000 }));
        }
        other => panic!("expected Comparison, got {other:?}"),
    }
}

#[test]
fn parse_step_with_when_or() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            step x {
                agent: a
                when: confidence < 70% or refund > $50.00
            }
        }
    "#,
    );
    let step = &f.workflows[0].steps[0];
    let when = step.when.as_ref().unwrap();
    match when {
        crate::ast::WhenExpr::Or(parts) => {
            assert_eq!(parts.len(), 2);
        }
        other => panic!("expected Or, got {other:?}"),
    }
}

#[test]
fn parse_step_with_when_and() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            step x {
                agent: a
                when: score >= 80 and priority < 3
            }
        }
    "#,
    );
    let step = &f.workflows[0].steps[0];
    let when = step.when.as_ref().unwrap();
    assert!(matches!(when, crate::ast::WhenExpr::And(parts) if parts.len() == 2));
}

#[test]
fn parse_step_without_when() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            step x { agent: a }
        }
    "#,
    );
    assert!(f.workflows[0].steps[0].when.is_none());
}

// ── retry policy tests ──────────────────────────────────────────────────

#[test]
fn parse_step_with_retry_escalate() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            step x {
                agent: a
                on failure: retry 3 exponential then escalate
            }
        }
    "#,
    );
    let step = &f.workflows[0].steps[0];
    let policy = step.on_failure.as_ref().unwrap();
    assert_eq!(policy.max_retries, 3);
    assert_eq!(policy.backoff, crate::ast::BackoffStrategy::Exponential);
    assert!(matches!(&policy.then, crate::ast::FailureAction::Escalate));
}

#[test]
fn parse_step_with_retry_linear_then_step() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            step x {
                agent: a
                on failure: retry 5 linear then fallback_handler
            }
        }
    "#,
    );
    let policy = f.workflows[0].steps[0].on_failure.as_ref().unwrap();
    assert_eq!(policy.max_retries, 5);
    assert_eq!(policy.backoff, crate::ast::BackoffStrategy::Linear);
    assert!(matches!(&policy.then, crate::ast::FailureAction::Step(s) if s == "fallback_handler"));
}

#[test]
fn parse_step_with_retry_fixed() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            step x {
                agent: a
                on failure: retry 2 fixed then escalate
            }
        }
    "#,
    );
    let policy = f.workflows[0].steps[0].on_failure.as_ref().unwrap();
    assert_eq!(policy.max_retries, 2);
    assert_eq!(policy.backoff, crate::ast::BackoffStrategy::Fixed);
}

#[test]
fn parse_step_with_when_and_retry() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            step x {
                agent: a
                when: score < 50
                on failure: retry 3 exponential then escalate
            }
        }
    "#,
    );
    let step = &f.workflows[0].steps[0];
    assert!(step.when.is_some());
    assert!(step.on_failure.is_some());
}

// ── auto resolve when tests ─────────────────────────────────────────────

#[test]
fn parse_auto_resolve_basic() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            step classify { agent: a }
            auto resolve when {
                confidence > 90%,
                action is one of [order_status, tracking]
            }
        }
    "#,
    );
    let ar = f.workflows[0].auto_resolve.as_ref().unwrap();
    assert_eq!(ar.conditions.len(), 2);

    match &ar.conditions[0] {
        crate::ast::AutoResolveCondition::Comparison(c) => {
            assert_eq!(c.field, "confidence");
            assert_eq!(c.op, crate::ast::CompareOp::Gt);
            assert!(matches!(&c.value, crate::ast::WhenValue::Percent(p) if p == "90"));
        }
        other => panic!("expected Comparison, got {other:?}"),
    }

    match &ar.conditions[1] {
        crate::ast::AutoResolveCondition::IsOneOf { field, variants } => {
            assert_eq!(field, "action");
            assert_eq!(variants, &["order_status", "tracking"]);
        }
        other => panic!("expected IsOneOf, got {other:?}"),
    }
}

#[test]
fn parse_auto_resolve_comparison_only() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            step s { agent: a }
            auto resolve when {
                score >= 95
            }
        }
    "#,
    );
    let ar = f.workflows[0].auto_resolve.as_ref().unwrap();
    assert_eq!(ar.conditions.len(), 1);
    assert!(matches!(&ar.conditions[0], crate::ast::AutoResolveCondition::Comparison(c) if c.field == "score"));
}

#[test]
fn parse_workflow_without_auto_resolve() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            step s { agent: a }
        }
    "#,
    );
    assert!(f.workflows[0].auto_resolve.is_none());
}

// ── when expression precedence tests ────────────────────────────────────

#[test]
fn when_and_binds_tighter_than_or() {
    // `a > 1 or b > 2 and c > 3` should parse as `a > 1 or (b > 2 and c > 3)`
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            step s {
                agent: a
                when: score > 50 or confidence > 80 and priority < 3
            }
        }
    "#,
    );
    let when = f.workflows[0].steps[0].when.as_ref().unwrap();
    match when {
        crate::ast::WhenExpr::Or(parts) => {
            assert_eq!(parts.len(), 2);
            // First part is a simple comparison
            assert!(matches!(&parts[0], crate::ast::WhenExpr::Comparison(_)));
            // Second part is an And group
            match &parts[1] {
                crate::ast::WhenExpr::And(and_parts) => assert_eq!(and_parts.len(), 2),
                other => panic!("expected And, got {other:?}"),
            }
        }
        other => panic!("expected Or, got {other:?}"),
    }
}

#[test]
fn when_and_chain_without_or() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            step s {
                agent: a
                when: a > 1 and b > 2 and c > 3
            }
        }
    "#,
    );
    let when = f.workflows[0].steps[0].when.as_ref().unwrap();
    assert!(matches!(when, crate::ast::WhenExpr::And(parts) if parts.len() == 3));
}

#[test]
fn when_or_chain_without_and() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            step s {
                agent: a
                when: a > 1 or b > 2 or c > 3
            }
        }
    "#,
    );
    let when = f.workflows[0].steps[0].when.as_ref().unwrap();
    assert!(matches!(when, crate::ast::WhenExpr::Or(parts) if parts.len() == 3));
}

#[test]
fn when_mixed_precedence_complex() {
    // `a > 1 and b > 2 or c > 3 and d > 4` = `(a > 1 and b > 2) or (c > 3 and d > 4)`
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            step s {
                agent: a
                when: x > 1 and y > 2 or z > 3 and w > 4
            }
        }
    "#,
    );
    let when = f.workflows[0].steps[0].when.as_ref().unwrap();
    match when {
        crate::ast::WhenExpr::Or(parts) => {
            assert_eq!(parts.len(), 2);
            assert!(matches!(&parts[0], crate::ast::WhenExpr::And(p) if p.len() == 2));
            assert!(matches!(&parts[1], crate::ast::WhenExpr::And(p) if p.len() == 2));
        }
        other => panic!("expected Or, got {other:?}"),
    }
}

// ── archetype/from tests ────────────────────────────────────────────────

#[test]
fn parse_archetype_basic() {
    let f = parse_ok(
        r#"
        archetype base {
            model: openai
            can [zendesk.read_ticket]
        }
        agent helper from base {
            budget: $0.05 per request
        }
        workflow w {
            trigger: event
            step s { agent: helper }
        }
    "#,
    );
    assert_eq!(f.archetypes.len(), 1);
    assert_eq!(f.archetypes[0].name, "base");
    assert_eq!(
        f.archetypes[0].model.as_ref().unwrap().display_value(),
        "openai"
    );
    assert_eq!(f.archetypes[0].can.len(), 1);

    assert_eq!(f.agents.len(), 1);
    assert_eq!(f.agents[0].name, "helper");
    assert_eq!(f.agents[0].from.as_deref(), Some("base"));
    assert!(f.agents[0].budget.is_some());
}

#[test]
fn parse_agent_without_from() {
    let f = parse_ok(
        r#"
        agent standalone { model: openai }
        workflow w { trigger: event step s { agent: standalone } }
    "#,
    );
    assert!(f.agents[0].from.is_none());
}

#[test]
fn parse_archetype_empty_body() {
    let f = parse_ok(
        r#"
        archetype empty {}
        agent a from empty { model: openai }
        workflow w { trigger: event step s { agent: a } }
    "#,
    );
    assert_eq!(f.archetypes[0].name, "empty");
    assert!(f.archetypes[0].model.is_none());
}

#[test]
fn parse_archetype_with_guardrails() {
    let f = parse_ok(
        r#"
        archetype guarded {
            model: anthropic
            guardrails {
                output_filter {
                    pii_detection: redact
                }
            }
        }
        agent bot from guarded {}
        workflow w { trigger: event step s { agent: bot } }
    "#,
    );
    assert!(f.archetypes[0].guardrails.is_some());
}

// ── policy/trust tests ──────────────────────────────────────────────────

#[test]
fn parse_policy_basic() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow w { trigger: event step s { agent: a } }
        policy {
            tier supervised {
                promote when accuracy > 95%
            }
            tier autonomous {}
        }
    "#,
    );
    assert_eq!(f.policies.len(), 1);
    assert_eq!(f.policies[0].tiers.len(), 2);
    assert_eq!(f.policies[0].tiers[0].name, "supervised");
    assert!(f.policies[0].tiers[0].promote_when.is_some());
    assert_eq!(f.policies[0].tiers[1].name, "autonomous");
    assert!(f.policies[0].tiers[1].promote_when.is_none());
}

#[test]
fn parse_policy_with_compound_condition() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow w { trigger: event step s { agent: a } }
        policy {
            tier monitored {
                promote when accuracy > 90% and errors < 5
            }
        }
    "#,
    );
    let tier = &f.policies[0].tiers[0];
    assert!(matches!(
        tier.promote_when.as_ref().unwrap(),
        crate::ast::WhenExpr::And(_)
    ));
}

#[test]
fn parse_policy_empty() {
    let f = parse_ok(
        r#"
        agent a { model: openai }
        workflow w { trigger: event step s { agent: a } }
        policy {}
    "#,
    );
    assert_eq!(f.policies[0].tiers.len(), 0);
}

#[test]
fn env_with_default_value() {
    let f = parse_ok(r#"provider openai { key: env("API_KEY", "fallback_key") }"#);
    match &f.providers[0].key {
        Some(crate::ast::ValueExpr::EnvRef {
            var_name, default, ..
        }) => {
            assert_eq!(var_name, "API_KEY");
            assert_eq!(default.as_deref(), Some("fallback_key"));
        }
        other => panic!("expected EnvRef with default, got: {other:?}"),
    }
}

#[test]
fn env_without_default_value() {
    let f = parse_ok(r#"provider openai { key: env("API_KEY") }"#);
    match &f.providers[0].key {
        Some(crate::ast::ValueExpr::EnvRef {
            var_name, default, ..
        }) => {
            assert_eq!(var_name, "API_KEY");
            assert_eq!(*default, None);
        }
        other => panic!("expected EnvRef without default, got: {other:?}"),
    }
}

#[test]
fn env_resolve_with_default_missing_var() {
    let expr = crate::ast::ValueExpr::EnvRef {
        var_name: "MISSING".to_string(),
        default: Some("fallback".to_string()),
        span: crate::ast::Span::new(0, 1),
    };
    let result = expr.resolve_with(|_| None).unwrap();
    assert_eq!(result, "fallback");
}

#[test]
fn env_resolve_with_default_present_var() {
    let expr = crate::ast::ValueExpr::EnvRef {
        var_name: "MY_VAR".to_string(),
        default: Some("fallback".to_string()),
        span: crate::ast::Span::new(0, 1),
    };
    let result = expr.resolve_with(|name| {
        if name == "MY_VAR" {
            Some("real_value".to_string())
        } else {
            None
        }
    }).unwrap();
    assert_eq!(result, "real_value");
}

#[test]
fn env_default_requires_string_literal() {
    let err = parse_err(r#"provider openai { key: env("API_KEY", 42) }"#);
    assert!(err.message.contains("string literal"), "got: {}", err.message);
}

#[test]
fn key_as_identifier_in_agent_name() {
    // "key" should be usable as an identifier since it's context-sensitive
    let f = parse_ok(r#"agent key { model: "gpt-4o" }"#);
    assert_eq!(f.agents[0].name, "key");
}

#[test]
fn key_field_still_works_in_provider() {
    let f = parse_ok(r#"provider openai { key: env("OPENAI_KEY") }"#);
    assert!(f.providers[0].key.is_some());
}

#[test]
fn inline_step_shorthand_with_goal() {
    let f = parse_ok(
        r#"
        agent triage { model: "gpt-4o" }
        workflow support {
            trigger: new_ticket
            step classify: triage goal "Classify this ticket"
        }
    "#,
    );
    assert_eq!(f.workflows[0].steps.len(), 1);
    let step = &f.workflows[0].steps[0];
    assert_eq!(step.name, "classify");
    assert_eq!(step.agent, "triage");
    assert_eq!(step.goal.as_deref(), Some("Classify this ticket"));
}

#[test]
fn inline_step_shorthand_without_goal() {
    let f = parse_ok(
        r#"
        agent triage { model: "gpt-4o" }
        workflow support {
            trigger: new_ticket
            step classify: triage
        }
    "#,
    );
    let step = &f.workflows[0].steps[0];
    assert_eq!(step.name, "classify");
    assert_eq!(step.agent, "triage");
    assert_eq!(step.goal, None);
}

#[test]
fn step_with_fallback() {
    let f = parse_ok(
        r#"
        agent primary { model: "gpt-4o" }
        agent backup { model: "gpt-4o" }
        workflow support {
            trigger: ticket
            step handle {
                agent: primary
                goal: "Handle the ticket"
                fallback step recover {
                    agent: backup
                    goal: "Recover from failure"
                }
            }
        }
    "#,
    );
    let step = &f.workflows[0].steps[0];
    assert_eq!(step.name, "handle");
    let fb = step.fallback.as_ref().expect("expected fallback");
    assert_eq!(fb.name, "recover");
    assert_eq!(fb.agent, "backup");
    assert_eq!(fb.goal.as_deref(), Some("Recover from failure"));
}

#[test]
fn step_duplicate_fallback_errors() {
    let err = parse_err(
        r#"
        workflow w {
            trigger: t
            step s {
                agent: a
                fallback step f1 { agent: b }
                fallback step f2 { agent: c }
            }
        }
    "#,
    );
    assert!(err.message.contains("duplicate 'fallback'"), "got: {}", err.message);
}

#[test]
fn pipe_expression_full() {
    let f = parse_ok(
        r#"
        agent a { model: "gpt-4o" }
        workflow w {
            trigger: event
            step analyze {
                agent: a
                input: products | where organic_traffic < 100 | sort by score desc | take 10
            }
        }
    "#,
    );
    let step = &f.workflows[0].steps[0];
    let pipe = step.input.as_ref().expect("expected input pipe");
    assert_eq!(pipe.source, "products");
    assert_eq!(pipe.transforms.len(), 3);
    match &pipe.transforms[0] {
        crate::ast::PipeTransform::Where { field, op, .. } => {
            assert_eq!(field, "organic_traffic");
            assert_eq!(*op, crate::ast::CompareOp::Lt);
        }
        other => panic!("expected Where, got: {other:?}"),
    }
    match &pipe.transforms[1] {
        crate::ast::PipeTransform::SortBy { field, direction } => {
            assert_eq!(field, "score");
            assert_eq!(*direction, crate::ast::SortDirection::Desc);
        }
        other => panic!("expected SortBy, got: {other:?}"),
    }
    match &pipe.transforms[2] {
        crate::ast::PipeTransform::Take { count } => assert_eq!(*count, 10),
        other => panic!("expected Take, got: {other:?}"),
    }
}

#[test]
fn pipe_expression_select_and_unique() {
    let f = parse_ok(
        r#"
        agent a { model: "gpt-4o" }
        workflow w {
            trigger: event
            step s {
                agent: a
                input: items | select name, price | unique name | skip 5
            }
        }
    "#,
    );
    let pipe = f.workflows[0].steps[0].input.as_ref().unwrap();
    assert_eq!(pipe.source, "items");
    assert_eq!(pipe.transforms.len(), 3);
    match &pipe.transforms[0] {
        crate::ast::PipeTransform::Select { fields } => {
            assert_eq!(fields, &["name", "price"]);
        }
        other => panic!("expected Select, got: {other:?}"),
    }
    match &pipe.transforms[1] {
        crate::ast::PipeTransform::Unique { field } => {
            assert_eq!(field.as_deref(), Some("name"));
        }
        other => panic!("expected Unique, got: {other:?}"),
    }
}

#[test]
fn observe_block() {
    let f = parse_ok(
        r#"
        observe support_metrics {
            trace: all steps
            metrics: [accuracy, latency, cost]
            alert when { accuracy < 90% }
            export: prometheus
        }
    "#,
    );
    assert_eq!(f.observes.len(), 1);
    let o = &f.observes[0];
    assert_eq!(o.name, "support_metrics");
    assert_eq!(o.trace.as_deref(), Some("all steps"));
    assert_eq!(o.metrics, vec!["accuracy", "latency", "cost"]);
    assert!(o.alert_when.is_some());
    assert_eq!(o.export.as_deref(), Some("prometheus"));
}

#[test]
fn fleet_block() {
    let f = parse_ok(
        r#"
        fleet support_team {
            agents: [agent_a, agent_b]
            policy: agent_trust
            budget: $500/day
            scaling {
                min: 2,
                max: 10
            }
        }
    "#,
    );
    assert_eq!(f.fleets.len(), 1);
    let fl = &f.fleets[0];
    assert_eq!(fl.name, "support_team");
    assert_eq!(fl.agents, vec!["agent_a", "agent_b"]);
    assert_eq!(fl.policy.as_deref(), Some("agent_trust"));
    assert_eq!(fl.budget, Some(50000));
    let sc = fl.scaling.as_ref().unwrap();
    assert_eq!(sc.min, 2);
    assert_eq!(sc.max, 10);
}

#[test]
fn channel_block() {
    let f = parse_ok(
        r#"
        channel pricing_updates {
            type: PriceChange[]
            retention: 7 days
        }
    "#,
    );
    assert_eq!(f.channels.len(), 1);
    let ch = &f.channels[0];
    assert_eq!(ch.name, "pricing_updates");
    assert_eq!(ch.message_type.as_deref(), Some("PriceChange[]"));
    assert_eq!(ch.retention.as_deref(), Some("7 days"));
}

#[test]
fn circuit_breaker_block() {
    let f = parse_ok(
        r#"
        circuit_breaker api_guard {
            open after: 5 failures in 10 min,
            half_open after: 2 min
        }
    "#,
    );
    assert_eq!(f.circuit_breakers.len(), 1);
    let cb = &f.circuit_breakers[0];
    assert_eq!(cb.name, "api_guard");
    assert_eq!(cb.failure_threshold, 5);
    assert_eq!(cb.window_minutes, 10);
    assert_eq!(cb.half_open_after_minutes, 2);
}

#[test]
fn step_with_send_to() {
    let f = parse_ok(
        r#"
        agent a { model: "gpt-4o" }
        workflow w {
            trigger: event
            step notify {
                agent: a
                send to: "slack(#pricing)"
                message: "{{count}} changes detected"
            }
        }
    "#,
    );
    let step = &f.workflows[0].steps[0];
    let send = step.send_to.as_ref().expect("expected send_to");
    assert_eq!(send.target, "slack(#pricing)");
    assert_eq!(
        send.message.as_deref(),
        Some("{{count}} changes detected")
    );
}

#[test]
fn within_constraint_block() {
    let f = parse_ok(
        r#"
        agent a { model: "gpt-4o" }
        workflow w {
            trigger: event
            within(cost: $0.05, latency: 2s) {
                step classify {
                    agent: a
                    goal: "Classify"
                }
            }
        }
    "#,
    );
    assert_eq!(f.workflows[0].within_blocks.len(), 1);
    let wb = &f.workflows[0].within_blocks[0];
    assert_eq!(wb.cost, Some(5));
    assert_eq!(wb.latency.as_deref(), Some("2s"));
    assert_eq!(wb.steps.len(), 1);
    assert_eq!(wb.steps[0].name, "classify");
}
