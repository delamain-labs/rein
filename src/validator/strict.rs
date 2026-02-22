use super::Diagnostic;
use crate::ast::{ReinFile, Span};

/// Features that parse and validate but are NOT enforced by the runtime.
/// In strict mode, we warn users about each one so they don't get a
/// false sense of security.
const UNENFORCED_FEATURES: &[(&str, &str)] = &[
    (
        "consensus",
        "Consensus blocks are parsed but not enforced. Multi-agent verification will not occur.",
    ),
    (
        "observe",
        "Observe blocks are parsed but not enforced. Custom metrics, alerts, and trace exports will not activate.",
    ),
    (
        "approval",
        "Approval workflow blocks are parsed but not enforced. Human-in-the-loop approvals will not gate execution.",
    ),
    (
        "secrets",
        "Secrets blocks are parsed but not enforced. Vault references will not be resolved.",
    ),
    (
        "fleet",
        "Fleet blocks are parsed but not enforced. Agent group scaling and policies will not apply.",
    ),
    (
        "channel",
        "Channel blocks are parsed but not enforced. Async agent messaging will not activate.",
    ),
    (
        "scenario",
        "Scenario blocks are parsed but not enforced. Declarative tests will not execute.",
    ),
];

/// Check for parsed-but-unenforced safety features and return warnings.
pub fn check_unenforced(file: &ReinFile) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let dummy_span = Span::new(0, 0);

    if !file.consensus_blocks.is_empty() {
        add_warning(&mut diags, "consensus", &dummy_span);
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
    if let Some((_, msg)) = UNENFORCED_FEATURES
        .iter()
        .find(|(name, _)| *name == feature)
    {
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
