use crate::ast::{ReinFile, Span};
use super::Diagnostic;

/// Features that parse and validate but are NOT enforced by the runtime.
/// In strict mode, we warn users about each one so they don't get a
/// false sense of security.
const UNENFORCED_FEATURES: &[(&str, &str)] = &[
    ("guardrails", "Guardrails blocks are parsed but not enforced at runtime. Output filtering, PII redaction, and toxicity blocking will not be applied."),
    ("policy", "Policy/trust tier blocks are parsed but not enforced. Progressive trust (promote/demote) will not take effect."),
    ("circuit_breaker", "Circuit breaker blocks are parsed but not enforced. Failure thresholds and half-open recovery will not activate."),
    ("consensus", "Consensus blocks are parsed but not enforced. Multi-agent verification will not occur."),
    ("eval", "Eval blocks are parsed but not enforced. Quality gates and dataset assertions will not run."),
    ("observe", "Observe blocks are parsed but not enforced. Custom metrics, alerts, and trace exports will not activate."),
    ("approval", "Approval workflow blocks are parsed but not enforced. Human-in-the-loop approvals will not gate execution."),
    ("secrets", "Secrets blocks are parsed but not enforced. Vault references will not be resolved."),
    ("fleet", "Fleet blocks are parsed but not enforced. Agent group scaling and policies will not apply."),
    ("channel", "Channel blocks are parsed but not enforced. Async agent messaging will not activate."),
    ("scenario", "Scenario blocks are parsed but not enforced. Declarative tests will not execute."),
];

/// Check for parsed-but-unenforced safety features and return warnings.
pub fn check_unenforced(file: &ReinFile) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let dummy_span = Span::new(0, 0);

    if !file.agents.iter().all(|a| a.guardrails.is_none()) {
        add_warning(&mut diags, "guardrails", &dummy_span);
    }
    if !file.policies.is_empty() {
        add_warning(&mut diags, "policy", &dummy_span);
    }
    if !file.circuit_breakers.is_empty() {
        add_warning(&mut diags, "circuit_breaker", &dummy_span);
    }
    if !file.consensus_blocks.is_empty() {
        add_warning(&mut diags, "consensus", &dummy_span);
    }
    if !file.evals.is_empty() {
        add_warning(&mut diags, "eval", &dummy_span);
    }
    if !file.observes.is_empty() {
        add_warning(&mut diags, "observe", &dummy_span);
    }
    if !file.secrets.is_empty() {
        add_warning(&mut diags, "secrets", &dummy_span);
    }
    if !file.fleets.is_empty() {
        add_warning(&mut diags, "fleet", &dummy_span);
    }
    if !file.channels.is_empty() {
        add_warning(&mut diags, "channel", &dummy_span);
    }
    if !file.scenarios.is_empty() {
        add_warning(&mut diags, "scenario", &dummy_span);
    }

    // Check for escalate in workflow steps
    for wf in &file.workflows {
        for step in &wf.steps {
            if step.escalate.is_some() {
                diags.push(Diagnostic::warning(
                    "W_UNENFORCED",
                    "Escalate keyword is parsed but not enforced at runtime. Human handoff will not occur.",
                    step.span.clone(),
                ));
                break;
            }
        }
    }

    diags
}

fn add_warning(diags: &mut Vec<Diagnostic>, feature: &str, span: &Span) {
    if let Some((_, msg)) = UNENFORCED_FEATURES.iter().find(|(name, _)| *name == feature) {
        diags.push(Diagnostic::warning("W_UNENFORCED", *msg, span.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_no_strict_warnings() {
        let file = ReinFile::default();
        let diags = check_unenforced(&file);
        assert!(diags.is_empty());
    }
}
