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
