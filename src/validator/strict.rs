use super::Diagnostic;
use crate::ast::ReinFile;

/// Check for parsed-but-unenforced safety features and return warnings.
///
/// As of v0.1, nearly all features are enforced at runtime. The only
/// remaining unenforced feature is `escalate` in workflow steps (human
/// handoff), which requires external integration.
pub fn check_unenforced(file: &ReinFile) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

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
