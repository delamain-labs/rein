use super::Diagnostic;
use crate::ast::ReinFile;

/// Check for parsed-but-unenforced safety features and return warnings.
///
/// Enforced at runtime: guardrails, circuit breakers, approval gates,
/// policy engine, budget limits, agent permissions, provider resolution.
///
/// Not yet enforced: consensus, observe, secrets, fleet, channel,
/// scenario, escalate. These parse correctly but have no runtime effect.
pub fn check_unenforced(file: &ReinFile) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    check_consensus(&file.consensus_blocks, &mut diags);
    check_observe(&file.observes, &mut diags);
    check_secrets(&file.secrets, &mut diags);
    check_fleets(&file.fleets, &mut diags);
    check_channels(&file.channels, &mut diags);
    check_scenarios(&file.scenarios, &mut diags);
    check_escalate(&file.workflows, &mut diags);

    diags
}

fn check_consensus(
    blocks: &[crate::ast::ConsensusDef],
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(first) = blocks.first() {
        diags.push(Diagnostic::warning(
            "W_UNENFORCED",
            "consensus blocks are parsed but not enforced at runtime. Multi-agent voting will not occur.",
            first.span.clone(),
        ));
    }
}

fn check_observe(
    blocks: &[crate::ast::ObserveDef],
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(first) = blocks.first() {
        diags.push(Diagnostic::warning(
            "W_UNENFORCED",
            "observe blocks are parsed but not enforced at runtime. Use `rein run --otel` for trace export.",
            first.span.clone(),
        ));
    }
}

fn check_secrets(
    blocks: &[crate::ast::SecretsDef],
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(first) = blocks.first() {
        diags.push(Diagnostic::warning(
            "W_UNENFORCED",
            "secrets blocks are parsed but not enforced at runtime. Secrets are not resolved from vaults.",
            first.span.clone(),
        ));
    }
}

fn check_fleets(
    blocks: &[crate::ast::FleetDef],
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(first) = blocks.first() {
        diags.push(Diagnostic::warning(
            "W_UNENFORCED",
            "fleet blocks are parsed but not enforced at runtime. Agent scaling will not occur.",
            first.span.clone(),
        ));
    }
}

fn check_channels(
    blocks: &[crate::ast::ChannelDef],
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(first) = blocks.first() {
        diags.push(Diagnostic::warning(
            "W_UNENFORCED",
            "channel blocks are parsed but not enforced at runtime. Messages will not be routed.",
            first.span.clone(),
        ));
    }
}

fn check_scenarios(
    blocks: &[crate::ast::ScenarioDef],
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(first) = blocks.first() {
        diags.push(Diagnostic::warning(
            "W_UNENFORCED",
            "scenario blocks are parsed but not enforced at runtime. Tests will not be executed.",
            first.span.clone(),
        ));
    }
}

fn check_escalate(
    workflows: &[crate::ast::WorkflowDef],
    diags: &mut Vec<Diagnostic>,
) {
    for wf in workflows {
        for step in &wf.steps {
            if step.escalate.is_some() {
                diags.push(Diagnostic::warning(
                    "W_UNENFORCED",
                    "escalate in steps is parsed but not enforced at runtime. Human handoff will not occur.",
                    step.span.clone(),
                ));
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn empty_file_no_strict_warnings() {
        let file = ReinFile::default();
        let diags = check_unenforced(&file);
        assert!(diags.is_empty());
    }

    #[test]
    fn warns_on_consensus_block() {
        let file = parse(
            r#"consensus panel { agents: [a, b] strategy: majority require: 2 of 3 agree }"#,
        )
        .unwrap();
        let diags = check_unenforced(&file);
        assert!(diags.iter().any(|d| d.message.contains("consensus")));
    }

    #[test]
    fn warns_on_observe_block() {
        let file = parse(r#"observe health { trace: "structured" }"#).unwrap();
        let diags = check_unenforced(&file);
        assert!(diags.iter().any(|d| d.message.contains("observe")));
    }

    #[test]
    fn warns_on_secrets_block() {
        let file =
            parse(r#"secrets { api_key: env("KEY") }"#).unwrap();
        let diags = check_unenforced(&file);
        assert!(diags.iter().any(|d| d.message.contains("secrets")));
    }

    #[test]
    fn warns_on_fleet_block() {
        let file = parse(r#"fleet team { agents: [a, b] }"#).unwrap();
        let diags = check_unenforced(&file);
        assert!(diags.iter().any(|d| d.message.contains("fleet")));
    }

    #[test]
    fn warns_on_channel_block() {
        let file = parse(r#"channel alerts { type: slack }"#).unwrap();
        let diags = check_unenforced(&file);
        assert!(diags.iter().any(|d| d.message.contains("channel")));
    }

    #[test]
    fn warns_on_scenario_block() {
        let file = parse(
            r#"scenario test { given { q: "hi" } expect { a: "hello" } }"#,
        )
        .unwrap();
        let diags = check_unenforced(&file);
        assert!(diags.iter().any(|d| d.message.contains("scenario")));
    }

    #[test]
    fn no_warning_on_enforced_features() {
        let file = parse(
            r#"
            agent bot { model: openai can [chat.respond] budget: $1 per request }
            circuit_breaker cb { open after: 3 failures in 5 min half_open after: 2 min }
            policy { tier supervised { promote when accuracy > 90% } }
        "#,
        )
        .unwrap();
        let diags = check_unenforced(&file);
        assert!(diags.is_empty());
    }
}
