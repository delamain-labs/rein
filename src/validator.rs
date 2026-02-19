use crate::ast::{AgentDef, ReinFile, Span};

/// Severity of a diagnostic.
#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

/// A validation diagnostic (error or warning) with source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    fn error(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Self {
            severity: Severity::Error,
            code,
            message: message.into(),
            span,
        }
    }

    fn warning(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Self {
            severity: Severity::Warning,
            code,
            message: message.into(),
            span,
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

/// Run all validation passes on a parsed file.
/// Returns a list of diagnostics (errors and warnings).
pub fn validate(file: &ReinFile) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    check_duplicate_agent_names(file, &mut diags);
    for agent in &file.agents {
        check_can_cannot_overlap(agent, &mut diags);
        check_budget_positive(agent, &mut diags);
        check_model_present(agent, &mut diags);
    }
    diags
}

/// E001: two agents with the same name.
fn check_duplicate_agent_names(file: &ReinFile, diags: &mut Vec<Diagnostic>) {
    use std::collections::HashMap;
    let mut seen: HashMap<&str, &AgentDef> = HashMap::new();
    for agent in &file.agents {
        if let Some(first) = seen.get(agent.name.as_str()) {
            diags.push(Diagnostic::error(
                "E001",
                format!(
                    "duplicate agent name '{}': first defined at {}",
                    agent.name, first.span.start
                ),
                agent.span.clone(),
            ));
        } else {
            seen.insert(agent.name.as_str(), agent);
        }
    }
}

/// E002: same tool appears in both `can` and `cannot`.
fn check_can_cannot_overlap(agent: &AgentDef, diags: &mut Vec<Diagnostic>) {
    for denied in &agent.cannot {
        for allowed in &agent.can {
            if allowed.namespace == denied.namespace && allowed.action == denied.action {
                diags.push(Diagnostic::error(
                    "E002",
                    format!(
                        "capability '{}.{}' appears in both `can` and `cannot` in agent '{}'",
                        denied.namespace, denied.action, agent.name
                    ),
                    denied.span.clone(),
                ));
            }
        }
    }
}

/// E003: budget amount must be positive (non-zero cents).
fn check_budget_positive(agent: &AgentDef, diags: &mut Vec<Diagnostic>) {
    if let Some(budget) = &agent.budget
        && budget.amount == 0
    {
        diags.push(Diagnostic::error(
            "E003",
            format!(
                "budget amount must be positive, got 0 in agent '{}'",
                agent.name
            ),
            budget.span.clone(),
        ));
    }
}

/// W001: agent has no `model` field.
fn check_model_present(agent: &AgentDef, diags: &mut Vec<Diagnostic>) {
    if agent.model.is_none() {
        diags.push(Diagnostic::warning(
            "W001",
            format!("agent '{}' has no `model` field", agent.name),
            agent.span.clone(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let file = ReinFile {
            agents: vec![AgentDef {
                name: "bot".into(),
                model: Some("anthropic".into()),
                can: vec![],
                cannot: vec![],
                budget: Some(Budget {
                    amount: 0,
                    currency: "USD".into(),
                    unit: "ticket".into(),
                    span: Span::new(0, 1),
                }),
                span: Span::new(0, 1),
            }],
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
}
