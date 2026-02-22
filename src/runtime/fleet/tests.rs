use super::*;
use crate::ast::{FleetDef, ScalingConfig, Span};

fn span() -> Span {
    Span { start: 0, end: 0 }
}

fn make_def() -> FleetDef {
    FleetDef {
        name: "support_fleet".to_string(),
        agents: vec!["triage".to_string(), "resolver".to_string()],
        policy: Some("cautious".to_string()),
        budget: Some(5000),
        scaling: Some(ScalingConfig {
            min: 2,
            max: 10,
            span: span(),
        }),
        span: span(),
    }
}

#[test]
fn creates_from_def() {
    let fleet = Fleet::from_def(&make_def());
    assert_eq!(fleet.name(), "support_fleet");
    assert_eq!(fleet.agents().len(), 2);
    assert_eq!(fleet.active_instances(), 2);
    assert_eq!(fleet.policy(), Some("cautious"));
    assert_eq!(fleet.budget_cents(), Some(5000));
}

#[test]
fn scale_up_increases_instances() {
    let mut fleet = Fleet::from_def(&make_def());
    let event = fleet.scale_up();
    assert_eq!(event, Some(FleetEvent::ScaledUp { from: 2, to: 3 }));
    assert_eq!(fleet.active_instances(), 3);
}

#[test]
fn scale_up_at_max_returns_at_capacity() {
    let mut def = make_def();
    def.scaling = Some(ScalingConfig {
        min: 2,
        max: 2,
        span: span(),
    });
    let mut fleet = Fleet::from_def(&def);
    let event = fleet.scale_up();
    assert_eq!(event, Some(FleetEvent::AtCapacity { current: 2, max: 2 }));
}

#[test]
fn scale_down_decreases_instances() {
    let mut fleet = Fleet::from_def(&make_def());
    fleet.scale_up(); // 2 -> 3
    let event = fleet.scale_down();
    assert_eq!(event, Some(FleetEvent::ScaledDown { from: 3, to: 2 }));
}

#[test]
fn scale_down_at_min_returns_at_minimum() {
    let mut fleet = Fleet::from_def(&make_def());
    let event = fleet.scale_down();
    assert_eq!(event, Some(FleetEvent::AtMinimum { current: 2, min: 2 }));
}

#[test]
fn no_scaling_config_defaults_to_one() {
    let def = FleetDef {
        name: "minimal".to_string(),
        agents: vec!["a".to_string()],
        policy: None,
        budget: None,
        scaling: None,
        span: span(),
    };
    let fleet = Fleet::from_def(&def);
    assert_eq!(fleet.active_instances(), 1);
}
