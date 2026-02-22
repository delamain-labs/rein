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
        from: None,
        name: "support_triage".to_string(),
        model: Some(ValueExpr::Literal("anthropic".into())),
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
        guardrails: None,
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
        archetypes: vec![],
        policies: vec![],
        observes: vec![],
        fleets: vec![],
        channels: vec![],
        circuit_breakers: vec![],
        evals: vec![],
        memories: vec![],
        secrets: vec![],
        consensus_blocks: vec![],
        scenarios: vec![],
        imports: vec![],
        defaults: None,
        providers: vec![],
        tools: vec![],
        agents: vec![AgentDef {
            from: None,
            name: "bot".to_string(),
            model: None,
            can: vec![],
            cannot: vec![],
            budget: None,
            guardrails: None,
            span: dummy_span(),
        }],
        workflows: vec![],
        types: vec![],
    };
    let json = serde_json::to_string(&file).unwrap();
    let decoded: ReinFile = serde_json::from_str(&json).unwrap();
    assert_eq!(file, decoded);
}

#[test]
fn agent_def_minimal_model_none() {
    let agent = AgentDef {
        from: None,
        name: "minimal".to_string(),
        model: None,
        can: vec![],
        cannot: vec![],
        budget: None,
        guardrails: None,
        span: dummy_span(),
    };
    let json = serde_json::to_value(&agent).unwrap();
    assert_eq!(json["model"], serde_json::Value::Null);
    assert!(json["can"].as_array().unwrap().is_empty());
}

// ── Workflow types ─────────────────────────────────────────────────

#[test]
fn stage_serializes() {
    let stage = Stage {
        name: "triage".to_string(),
        agent: "support_triage".to_string(),
        route: RouteRule::Next,
        span: dummy_span(),
    };
    let json = serde_json::to_value(&stage).unwrap();
    assert_eq!(json["name"], "triage");
    assert_eq!(json["agent"], "support_triage");
    assert_eq!(json["route"]["type"], "next");
}

#[test]
fn conditional_route_serializes() {
    let route = RouteRule::Conditional {
        field: "sentiment".to_string(),
        matcher: ConditionMatcher::Equals("negative".to_string()),
        then_stage: "escalate".to_string(),
        else_stage: Some("respond".to_string()),
    };
    let json = serde_json::to_value(&route).unwrap();
    assert_eq!(json["type"], "conditional");
    assert_eq!(json["field"], "sentiment");
    assert_eq!(json["then_stage"], "escalate");
}

#[test]
fn workflow_def_serializes() {
    let workflow = WorkflowDef {
        name: "support_pipeline".to_string(),
        trigger: "incoming_ticket".to_string(),
        stages: vec![
            Stage {
                name: "triage".to_string(),
                agent: "support_triage".to_string(),
                route: RouteRule::Next,
                span: dummy_span(),
            },
            Stage {
                name: "respond".to_string(),
                agent: "responder".to_string(),
                route: RouteRule::Next,
                span: dummy_span(),
            },
        ],
        steps: vec![],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: None,
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: dummy_span(),
    };
    let json = serde_json::to_value(&workflow).unwrap();
    assert_eq!(json["name"], "support_pipeline");
    assert_eq!(json["trigger"], "incoming_ticket");
    assert_eq!(json["stages"].as_array().unwrap().len(), 2);
    assert_eq!(json["mode"], "sequential");
}

#[test]
fn workflow_roundtrips_via_json() {
    let workflow = WorkflowDef {
        name: "test".to_string(),
        trigger: "event".to_string(),
        stages: vec![],
        steps: vec![],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: None,
        within_blocks: vec![],
        mode: ExecutionMode::Parallel,
        schedule: None,
        span: dummy_span(),
    };
    let json = serde_json::to_string(&workflow).unwrap();
    let decoded: WorkflowDef = serde_json::from_str(&json).unwrap();
    assert_eq!(workflow, decoded);
}

#[test]
fn rein_file_with_workflows_roundtrips() {
    let file = ReinFile {
        archetypes: vec![],
        policies: vec![],
        observes: vec![],
        fleets: vec![],
        channels: vec![],
        circuit_breakers: vec![],
        evals: vec![],
        memories: vec![],
        secrets: vec![],
        consensus_blocks: vec![],
        scenarios: vec![],
        imports: vec![],
        defaults: None,
        providers: vec![],
        tools: vec![],
        agents: vec![],
        workflows: vec![WorkflowDef {
            name: "pipeline".to_string(),
            trigger: "event".to_string(),
            stages: vec![],
            steps: vec![],
            route_blocks: vec![],
            parallel_blocks: vec![],
            auto_resolve: None,
            within_blocks: vec![],
            mode: ExecutionMode::Sequential,
            schedule: None,
            span: dummy_span(),
        }],
        types: vec![],
    };
    let json = serde_json::to_string(&file).unwrap();
    let decoded: ReinFile = serde_json::from_str(&json).unwrap();
    assert_eq!(file, decoded);
}

#[test]
fn workflow_def_find_stage() {
    let wf = WorkflowDef {
        name: "test".to_string(),
        trigger: "event".to_string(),
        stages: vec![
            Stage {
                name: "a".to_string(),
                agent: "agent_a".to_string(),
                route: RouteRule::Next,
                span: dummy_span(),
            },
            Stage {
                name: "b".to_string(),
                agent: "agent_b".to_string(),
                route: RouteRule::Next,
                span: dummy_span(),
            },
        ],
        steps: vec![],
        route_blocks: vec![],
        parallel_blocks: vec![],
        auto_resolve: None,
        within_blocks: vec![],
        mode: ExecutionMode::Sequential,
        schedule: None,
        span: dummy_span(),
    };

    assert_eq!(wf.find_stage("a").unwrap().agent, "agent_a");
    assert_eq!(wf.find_stage("b").unwrap().agent, "agent_b");
    assert!(wf.find_stage("nonexistent").is_none());
}
