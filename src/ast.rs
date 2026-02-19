use serde::{Deserialize, Serialize};

/// Byte-offset span in the source file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// A monetary cap constraint on a capability (`up to $<amount>`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Constraint {
    MonetaryCap { amount: u64, currency: String },
}

/// A single tool capability, e.g. `zendesk.read_ticket` or `zendesk.refund up to $50`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Capability {
    pub namespace: String,
    pub action: String,
    pub constraint: Option<Constraint>,
    pub span: Span,
}

/// A spending budget, e.g. `budget: $0.03 per ticket`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Budget {
    pub amount: u64,
    pub currency: String,
    pub unit: String,
    pub span: Span,
}

/// A single `agent <name> { ... }` definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentDef {
    pub name: String,
    pub model: Option<String>,
    pub can: Vec<Capability>,
    pub cannot: Vec<Capability>,
    pub budget: Option<Budget>,
    pub span: Span,
}

/// Top-level parsed file — one or more agent definitions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReinFile {
    pub agents: Vec<AgentDef>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_span() -> Span {
        Span::new(0, 1)
    }

    #[test]
    fn span_serializes_to_json() {
        let span = Span::new(10, 20);
        let json = serde_json::to_string(&span).unwrap();
        assert_eq!(json, r#"{"start":10,"end":20}"#);
    }

    #[test]
    fn constraint_monetary_cap_serializes() {
        let c = Constraint::MonetaryCap {
            amount: 5000,
            currency: "USD".to_string(),
        };
        let json = serde_json::to_value(&c).unwrap();
        assert_eq!(json["type"], "MonetaryCap");
        assert_eq!(json["amount"], 5000);
        assert_eq!(json["currency"], "USD");
    }

    #[test]
    fn capability_with_constraint_serializes() {
        let cap = Capability {
            namespace: "zendesk".to_string(),
            action: "refund".to_string(),
            constraint: Some(Constraint::MonetaryCap {
                amount: 5000,
                currency: "USD".to_string(),
            }),
            span: dummy_span(),
        };
        let json = serde_json::to_value(&cap).unwrap();
        assert_eq!(json["namespace"], "zendesk");
        assert_eq!(json["action"], "refund");
        assert_eq!(json["constraint"]["amount"], 5000);
    }

    #[test]
    fn capability_without_constraint_serializes() {
        let cap = Capability {
            namespace: "zendesk".to_string(),
            action: "read_ticket".to_string(),
            constraint: None,
            span: dummy_span(),
        };
        let json = serde_json::to_value(&cap).unwrap();
        assert_eq!(json["constraint"], serde_json::Value::Null);
    }

    #[test]
    fn budget_serializes() {
        let b = Budget {
            amount: 3,
            currency: "USD".to_string(),
            unit: "ticket".to_string(),
            span: dummy_span(),
        };
        let json = serde_json::to_value(&b).unwrap();
        assert_eq!(json["amount"], 3);
        assert_eq!(json["currency"], "USD");
        assert_eq!(json["unit"], "ticket");
    }

    #[test]
    fn agent_def_full_serializes() {
        let agent = AgentDef {
            name: "support_triage".to_string(),
            model: Some("anthropic".to_string()),
            can: vec![Capability {
                namespace: "zendesk".to_string(),
                action: "read_ticket".to_string(),
                constraint: None,
                span: dummy_span(),
            }],
            cannot: vec![Capability {
                namespace: "zendesk".to_string(),
                action: "delete_ticket".to_string(),
                constraint: None,
                span: dummy_span(),
            }],
            budget: Some(Budget {
                amount: 3,
                currency: "USD".to_string(),
                unit: "ticket".to_string(),
                span: dummy_span(),
            }),
            span: dummy_span(),
        };
        let json = serde_json::to_value(&agent).unwrap();
        assert_eq!(json["name"], "support_triage");
        assert_eq!(json["model"], "anthropic");
        assert_eq!(json["can"].as_array().unwrap().len(), 1);
        assert_eq!(json["cannot"].as_array().unwrap().len(), 1);
        assert_eq!(json["budget"]["unit"], "ticket");
    }

    #[test]
    fn rein_file_roundtrips_via_json() {
        let file = ReinFile {
            agents: vec![AgentDef {
                name: "bot".to_string(),
                model: None,
                can: vec![],
                cannot: vec![],
                budget: None,
                span: dummy_span(),
            }],
        };
        let json = serde_json::to_string(&file).unwrap();
        let decoded: ReinFile = serde_json::from_str(&json).unwrap();
        assert_eq!(file, decoded);
    }

    #[test]
    fn agent_def_minimal_model_none() {
        let agent = AgentDef {
            name: "minimal".to_string(),
            model: None,
            can: vec![],
            cannot: vec![],
            budget: None,
            span: dummy_span(),
        };
        let json = serde_json::to_value(&agent).unwrap();
        assert_eq!(json["model"], serde_json::Value::Null);
        assert!(json["can"].as_array().unwrap().is_empty());
    }
}
