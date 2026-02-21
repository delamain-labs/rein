use super::*;
use crate::ast::ValueExpr;
use crate::parser::parse;

fn validate_src(src: &str) -> Vec<Diagnostic> {
    let file = parse(src).expect("parse should succeed");
    validate(&file)
}

fn errors(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    diags.iter().filter(|d| d.is_error()).collect()
}

fn warnings(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    diags.iter().filter(|d| !d.is_error()).collect()
}

// ── Duplicate agent names (E001) ──────────────────────────────────────────

#[test]
fn duplicate_agent_names_detected() {
    let src = r#"
agent support { model: anthropic }
agent support { model: openai }
"#;
    let diags = validate_src(src);
    let errs = errors(&diags);
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].code, "E001");
    assert!(errs[0].message.contains("support"));
}

#[test]
fn unique_agent_names_ok() {
    let src = r#"
agent alpha { model: anthropic }
agent beta  { model: openai }
"#;
    let diags = validate_src(src);
    assert!(errors(&diags).is_empty(), "expected no errors");
}

// ── Can/cannot overlap (E002) ─────────────────────────────────────────────

#[test]
fn same_tool_in_can_and_cannot_detected() {
    let src = r#"
agent foo {
model: anthropic
can    [ zendesk.read_ticket ]
cannot [ zendesk.read_ticket ]
}
"#;
    let diags = validate_src(src);
    let errs = errors(&diags);
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].code, "E002");
    assert!(errs[0].message.contains("read_ticket"));
}

#[test]
fn no_overlap_ok() {
    let src = r#"
agent foo {
model: anthropic
can    [ zendesk.read_ticket ]
cannot [ zendesk.delete_ticket ]
}
"#;
    let diags = validate_src(src);
    assert!(errors(&diags).is_empty());
}

// ── Budget positive (E003) ────────────────────────────────────────────────

#[test]
fn zero_budget_detected() {
    // We can't express $0 directly in the grammar, so we build the AST
    // directly. amount is u64 (cents), so 0 is the only invalid value.
    use crate::ast::{AgentDef, Budget, ReinFile, Span};
    let file = ReinFile { archetypes: vec![], policies: vec![],
            observes: vec![], fleets: vec![], channels: vec![],
            imports: vec![],
        defaults: None,
        providers: vec![],
        tools: vec![],
        agents: vec![AgentDef { from: None,
            name: "bot".into(),
            model: Some(ValueExpr::Literal("anthropic".into())),
            can: vec![],
            cannot: vec![],
            budget: Some(Budget {
                amount: 0,
                currency: "USD".into(),
                unit: "ticket".into(),
                span: Span::new(0, 1),
            }),
            guardrails: None,
            span: Span::new(0, 1),
        }],
        workflows: vec![],
        types: vec![],
    };
    let diags = validate(&file);
    let errs = errors(&diags);
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].code, "E003");
}

// NOTE: negative budgets are impossible to represent — amount is u64 (cents).

#[test]
fn positive_budget_ok() {
    let src = "agent foo { model: anthropic  budget: $0.03 per ticket }";
    let diags = validate_src(src);
    assert!(errors(&diags).is_empty());
}

// ── Missing model warning (W001) ──────────────────────────────────────────

#[test]
fn missing_model_produces_warning() {
    let src = "agent foo { }";
    let diags = validate_src(src);
    let warns = warnings(&diags);
    assert_eq!(warns.len(), 1);
    assert_eq!(warns[0].code, "W001");
    assert!(warns[0].message.contains("foo"));
}

#[test]
fn present_model_no_warning() {
    let src = "agent foo { model: anthropic }";
    let diags = validate_src(src);
    assert!(warnings(&diags).is_empty());
}

// ── Constraint amount validation ────────────────────────────────────────────

#[test]
fn zero_constraint_amount_produces_error() {
    let src = "agent foo { can [ billing.refund up to $0 ] }";
    let diags = validate_src(src);
    let errs = errors(&diags);
    assert!(
        errs.iter().any(|d| d.code == "E004"),
        "expected E004, got: {:?}",
        errs
    );
}

#[test]
fn positive_constraint_amount_no_error() {
    let src = "agent foo { can [ billing.refund up to $50 ] }";
    let diags = validate_src(src);
    assert!(!diags.iter().any(|d| d.code == "E004"));
}

// ── Duplicate capabilities ─────────────────────────────────────────────────

#[test]
fn duplicate_capability_produces_warning() {
    let src = "agent foo { can [ zendesk.read_ticket zendesk.read_ticket ] }";
    let diags = validate_src(src);
    let warns = warnings(&diags);
    assert!(
        warns.iter().any(|d| d.code == "W003"),
        "expected W003, got: {:?}",
        warns
    );
}

#[test]
fn no_duplicate_capability_no_warning() {
    let src = "agent foo { can [ zendesk.read_ticket zendesk.refund ] }";
    let diags = validate_src(src);
    assert!(!diags.iter().any(|d| d.code == "W003"));
}

#[test]
fn duplicate_in_cannot_list() {
    let src = "agent foo { cannot [ stripe.charge stripe.charge ] }";
    let diags = validate_src(src);
    assert!(diags.iter().any(|d| d.code == "W003"));
}

// ── Multiple errors at once ───────────────────────────────────────────────

#[test]
fn multiple_errors_reported() {
    let src = r#"
agent dup { }
agent dup { }
"#;
    let diags = validate_src(src);
    // E001 duplicate + W001 for each dup agent (2 agents × W001) + one E001
    assert!(diags.iter().any(|d| d.code == "E001"));
    assert!(diags.iter().any(|d| d.code == "W001"));
}

// ── Workflow validation ──────────────────────────────────────────────────

#[test]
fn workflow_valid_stages_no_errors() {
    let diags = validate_src(
        r#"
        agent triage { model: openai }
        agent responder { model: openai }
        workflow pipe {
            trigger: event
            stages: [triage, responder]
        }
    "#,
    );
    let errs = errors(&diags);
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
}

#[test]
fn workflow_unknown_agent_errors() {
    let diags = validate_src(
        r#"
        agent triage { model: openai }
        workflow pipe {
            trigger: event
            stages: [triage, nonexistent]
        }
    "#,
    );
    let errs = errors(&diags);
    assert_eq!(errs.len(), 1);
    assert!(
        errs[0].message.contains("nonexistent"),
        "msg: {}",
        errs[0].message
    );
    assert_eq!(errs[0].code, "E006");
}

#[test]
fn workflow_duplicate_names_errors() {
    let diags = validate_src(
        r#"
        agent a { model: openai }
        workflow pipe { trigger: e1 stages: [a] }
        workflow pipe { trigger: e2 stages: [a] }
    "#,
    );
    let errs = errors(&diags);
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].code, "E005");
}

#[test]
fn workflow_duplicate_stages_warns() {
    let diags = validate_src(
        r#"
        agent a { model: openai }
        workflow pipe {
            trigger: event
            stages: [a, a]
        }
    "#,
    );
    let warns = warnings(&diags);
    assert_eq!(warns.len(), 1);
    assert_eq!(warns[0].code, "W004");
}

// ── Provider validation tests ─────────────────────────────────────────────

#[test]
fn duplicate_provider_names_error() {
    let diags = validate_src(
        r#"
        provider openai { model: "gpt-4o" key: env("K1") }
        provider openai { model: "gpt-4o-mini" key: env("K2") }
    "#,
    );
    let errors: Vec<_> = diags.iter().filter(|d| d.code == "E007").collect();
    assert_eq!(errors.len(), 1);
}

#[test]
fn provider_missing_key_warns() {
    let diags = validate_src("provider openai { model: openai }");
    let warns: Vec<_> = diags.iter().filter(|d| d.code == "W005").collect();
    assert_eq!(warns.len(), 1);
}

#[test]
fn provider_with_key_no_warning() {
    let diags = validate_src(r#"provider openai { model: openai key: env("K") }"#);
    let warns: Vec<_> = diags.iter().filter(|d| d.code == "W005").collect();
    assert_eq!(warns.len(), 0);
}

// ── Step validation tests ─────────────────────────────────────────────────

#[test]
fn step_references_unknown_agent_errors() {
    let diags = validate_src(
        r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            step s1 {
                agent: nonexistent
            }
        }
    "#,
    );
    let errors: Vec<_> = diags.iter().filter(|d| d.code == "E008").collect();
    assert_eq!(errors.len(), 1);
}

#[test]
fn step_references_valid_agent_ok() {
    let diags = validate_src(
        r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            step s1 {
                agent: a
            }
        }
    "#,
    );
    let errors: Vec<_> = diags.iter().filter(|d| d.code == "E008").collect();
    assert_eq!(errors.len(), 0);
}

#[test]
fn duplicate_step_names_error() {
    let diags = validate_src(
        r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            step s1 { agent: a }
            step s1 { agent: a }
        }
    "#,
    );
    let errors: Vec<_> = diags.iter().filter(|d| d.code == "E009").collect();
    assert_eq!(errors.len(), 1);
}

#[test]
fn duplicate_tool_names_error() {
    let diags = validate_src(
        r#"
        tool zendesk { endpoint: "https://a.com" }
        tool zendesk { endpoint: "https://b.com" }
    "#,
    );
    let errors: Vec<_> = diags.iter().filter(|d| d.code == "E011").collect();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("duplicate tool name"));
}

#[test]
fn step_stage_name_collision_errors() {
    let diags = validate_src(
        r#"
        agent a { model: openai }
        workflow w {
            trigger: event
            stages: [a]
            step a { agent: a }
        }
    "#,
    );
    let errors: Vec<_> = diags.iter().filter(|d| d.code == "E010").collect();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("collides with stage"));
}

#[test]
fn model_with_known_provider_prefix_no_warning() {
    let diags = validate_src(
        r#"
        provider anthropic { key: "k" }
        agent foo { model: "anthropic/claude-3" }
    "#,
    );
    let w006: Vec<_> = diags.iter().filter(|d| d.code == "W006").collect();
    assert!(w006.is_empty(), "expected no W006, got: {w006:?}");
}

#[test]
fn model_with_unknown_provider_prefix_warns() {
    let diags = validate_src(
        r#"
        agent foo { model: "unknown_provider/some-model" }
    "#,
    );
    let w006: Vec<_> = diags.iter().filter(|d| d.code == "W006").collect();
    assert_eq!(w006.len(), 1);
    assert!(w006[0].message.contains("unknown_provider"));
}

#[test]
fn model_without_slash_no_provider_warning() {
    let diags = validate_src(
        r#"
        agent foo { model: "gpt-4o" }
    "#,
    );
    let w006: Vec<_> = diags.iter().filter(|d| d.code == "W006").collect();
    assert!(w006.is_empty());
}
