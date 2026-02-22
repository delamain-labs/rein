use crate::ast::GuardrailsDef;

#[cfg(test)]
mod tests;

/// The action to take when a guardrail is triggered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardrailAction {
    /// Block the output entirely.
    Block,
    /// Redact the matched content.
    Redact,
    /// Log a warning but allow the output.
    Warn,
}

/// A compiled guardrail rule ready for enforcement.
#[derive(Debug, Clone)]
pub struct CompiledRule {
    pub key: String,
    pub action: GuardrailAction,
    pub section: String,
}

/// Result of applying guardrails to a piece of text.
#[derive(Debug, Clone)]
pub struct GuardrailResult {
    /// Whether the output was blocked entirely.
    pub blocked: bool,
    /// The (possibly redacted) output text.
    pub output: String,
    /// Warnings or violations that were triggered.
    pub violations: Vec<GuardrailViolation>,
}

/// A single guardrail violation.
#[derive(Debug, Clone)]
pub struct GuardrailViolation {
    pub rule_key: String,
    pub section: String,
    pub action: GuardrailAction,
    pub detail: String,
}

/// The guardrail engine. Holds compiled rules from parsed `.rein` guardrail blocks
/// and applies them to LLM outputs.
#[derive(Debug)]
pub struct GuardrailEngine {
    rules: Vec<CompiledRule>,
}

impl GuardrailEngine {
    /// Create an engine from parsed guardrail definitions.
    #[must_use]
    pub fn from_def(def: &GuardrailsDef) -> Self {
        let rules = def
            .sections
            .iter()
            .flat_map(|section| {
                section.rules.iter().map(move |rule| CompiledRule {
                    key: rule.key.clone(),
                    action: parse_action(&rule.value),
                    section: section.name.clone(),
                })
            })
            .collect();
        Self { rules }
    }

    /// Create an empty engine (no guardrails).
    #[must_use]
    pub fn empty() -> Self {
        Self { rules: Vec::new() }
    }

    /// Returns true if there are no rules configured.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Apply all guardrail rules to the given LLM output text.
    /// Returns the result with possible redactions or blocking.
    #[must_use]
    pub fn apply(&self, text: &str) -> GuardrailResult {
        let mut output = text.to_string();
        let mut violations = Vec::new();
        let mut blocked = false;

        for rule in &self.rules {
            if let Some(violation) = check_rule(rule, &output) {
                match violation.action {
                    GuardrailAction::Block => {
                        blocked = true;
                        violations.push(violation);
                    }
                    GuardrailAction::Redact => {
                        output = apply_redaction(&rule.key, &output);
                        violations.push(violation);
                    }
                    GuardrailAction::Warn => {
                        violations.push(violation);
                    }
                }
            }
        }

        GuardrailResult {
            blocked,
            output,
            violations,
        }
    }
}

/// Parse an action string from the DSL (e.g., "block", "redact", "warn").
fn parse_action(value: &str) -> GuardrailAction {
    match value.to_lowercase().as_str() {
        "block" => GuardrailAction::Block,
        "redact" => GuardrailAction::Redact,
        _ => GuardrailAction::Warn,
    }
}

/// Check a single rule against text. Returns a violation if triggered.
fn check_rule(rule: &CompiledRule, text: &str) -> Option<GuardrailViolation> {
    let triggered = match rule.key.as_str() {
        "pii_detection" => detect_pii(text),
        "toxicity" => detect_toxicity(text),
        "prompt_injection" => detect_prompt_injection(text),
        "code_execution" => detect_code_execution(text),
        _ => false,
    };

    if triggered {
        Some(GuardrailViolation {
            rule_key: rule.key.clone(),
            section: rule.section.clone(),
            action: rule.action.clone(),
            detail: format!(
                "Guardrail '{}' triggered in section '{}'",
                rule.key, rule.section
            ),
        })
    } else {
        None
    }
}

/// Basic PII detection: emails, phone numbers, SSNs.
fn detect_pii(text: &str) -> bool {
    // Email pattern
    if text.contains('@') && text.contains('.') {
        let words: Vec<&str> = text.split_whitespace().collect();
        for word in &words {
            if word.contains('@') && word.contains('.') && word.len() > 5 {
                return true;
            }
        }
    }

    // SSN pattern: NNN-NN-NNNN
    let chars: Vec<char> = text.chars().collect();
    for window in chars.windows(11) {
        let s: String = window.iter().collect();
        if is_ssn_pattern(&s) {
            return true;
        }
    }

    // Phone pattern: sequences of 10+ digits
    let digit_count = text.chars().filter(char::is_ascii_digit).count();
    if digit_count >= 10 {
        let mut consecutive = 0u32;
        for c in text.chars() {
            if c.is_ascii_digit() || c == '-' || c == ' ' || c == '(' || c == ')' {
                if c.is_ascii_digit() {
                    consecutive += 1;
                }
            } else {
                consecutive = 0;
            }
            if consecutive >= 10 {
                return true;
            }
        }
    }

    false
}

/// Check if a string matches SSN format: NNN-NN-NNNN.
fn is_ssn_pattern(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() != 11 {
        return false;
    }
    bytes[0].is_ascii_digit()
        && bytes[1].is_ascii_digit()
        && bytes[2].is_ascii_digit()
        && bytes[3] == b'-'
        && bytes[4].is_ascii_digit()
        && bytes[5].is_ascii_digit()
        && bytes[6] == b'-'
        && bytes[7].is_ascii_digit()
        && bytes[8].is_ascii_digit()
        && bytes[9].is_ascii_digit()
        && bytes[10].is_ascii_digit()
}

/// Basic toxicity detection via keyword matching.
fn detect_toxicity(text: &str) -> bool {
    let lower = text.to_lowercase();
    let toxic_patterns = [
        "kill yourself",
        "harm yourself",
        "self-harm",
        "suicide instructions",
        "how to make a bomb",
        "how to make explosives",
    ];
    toxic_patterns.iter().any(|p| lower.contains(p))
}

/// Basic prompt injection detection.
fn detect_prompt_injection(text: &str) -> bool {
    let lower = text.to_lowercase();
    let injection_patterns = [
        "ignore previous instructions",
        "ignore all instructions",
        "disregard your instructions",
        "override your system prompt",
        "you are now",
        "new instructions:",
        "system: you are",
    ];
    injection_patterns.iter().any(|p| lower.contains(p))
}

/// Detect potentially dangerous code execution patterns.
fn detect_code_execution(text: &str) -> bool {
    let lower = text.to_lowercase();
    let dangerous_patterns = [
        "rm -rf /",
        "sudo rm",
        "format c:",
        "del /f /s /q",
        "; drop table",
        "' or 1=1",
        "<script>",
    ];
    dangerous_patterns.iter().any(|p| lower.contains(p))
}

/// Apply redaction to the text for a given rule type.
fn apply_redaction(rule_key: &str, text: &str) -> String {
    match rule_key {
        "pii_detection" => redact_pii(text),
        _ => text.to_string(),
    }
}

/// Redact PII patterns from text.
fn redact_pii(text: &str) -> String {
    let mut result = text.to_string();

    // Redact email-like patterns.
    let words: Vec<&str> = text.split_whitespace().collect();
    for word in &words {
        if word.contains('@') && word.contains('.') && word.len() > 5 {
            result = result.replace(word, "[REDACTED_EMAIL]");
        }
    }

    // Redact SSN patterns.
    let chars: Vec<char> = result.chars().collect();
    let mut redacted = String::with_capacity(result.len());
    let mut i = 0;
    while i < chars.len() {
        if i + 11 <= chars.len() {
            let candidate: String = chars[i..i + 11].iter().collect();
            if is_ssn_pattern(&candidate) {
                redacted.push_str("[REDACTED_SSN]");
                i += 11;
                continue;
            }
        }
        redacted.push(chars[i]);
        i += 1;
    }

    redacted
}
