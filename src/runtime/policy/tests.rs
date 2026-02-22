use super::*;
use crate::ast::{CompareOp, PolicyDef, PolicyTier, Span, WhenComparison, WhenExpr, WhenValue};

fn span() -> Span {
    Span { start: 0, end: 0 }
}

fn make_policy(tiers: Vec<(&str, Option<(&str, f64)>)>) -> PolicyDef {
    PolicyDef {
        tiers: tiers
            .into_iter()
            .map(|(name, promote)| PolicyTier {
                name: name.to_string(),
                promote_when: promote.map(|(metric, threshold)| {
                    WhenExpr::Comparison(WhenComparison {
                        field: metric.to_string(),
                        op: CompareOp::Gt,
                        value: WhenValue::Percent(threshold.to_string()),
                    })
                }),
                span: span(),
            })
            .collect(),
        span: span(),
    }
}

#[test]
fn starts_at_first_tier() {
    let def = make_policy(vec![
        ("supervised", Some(("accuracy", 95.0))),
        ("autonomous", None),
    ]);
    let engine = PolicyEngine::from_def(&def);
    assert_eq!(engine.current_tier(), "supervised");
    assert_eq!(engine.current_tier_index(), 0);
}

#[test]
fn promotes_when_metric_met() {
    let def = make_policy(vec![
        ("supervised", Some(("accuracy", 95.0))),
        ("autonomous", None),
    ]);
    let mut engine = PolicyEngine::from_def(&def);
    let metrics = vec![("accuracy".to_string(), 96.0)];
    let event = engine.evaluate_promotion(&metrics);
    assert!(event.is_some());
    let event = event.unwrap();
    assert_eq!(event.from_tier, "supervised");
    assert_eq!(event.to_tier, "autonomous");
    assert_eq!(engine.current_tier(), "autonomous");
}

#[test]
fn does_not_promote_below_threshold() {
    let def = make_policy(vec![
        ("supervised", Some(("accuracy", 95.0))),
        ("autonomous", None),
    ]);
    let mut engine = PolicyEngine::from_def(&def);
    let metrics = vec![("accuracy".to_string(), 80.0)];
    assert!(engine.evaluate_promotion(&metrics).is_none());
    assert_eq!(engine.current_tier(), "supervised");
}

#[test]
fn does_not_promote_past_max_tier() {
    let def = make_policy(vec![
        ("supervised", Some(("accuracy", 95.0))),
        ("autonomous", None),
    ]);
    let mut engine = PolicyEngine::from_def(&def);
    let metrics = vec![("accuracy".to_string(), 99.0)];
    engine.evaluate_promotion(&metrics);
    assert!(engine.is_max_tier());
    assert!(engine.evaluate_promotion(&metrics).is_none());
}

#[test]
fn demote_drops_one_tier() {
    let def = make_policy(vec![
        ("supervised", Some(("accuracy", 95.0))),
        ("autonomous", None),
    ]);
    let mut engine = PolicyEngine::from_def(&def);
    let metrics = vec![("accuracy".to_string(), 99.0)];
    engine.evaluate_promotion(&metrics);
    assert_eq!(engine.current_tier(), "autonomous");

    let event = engine.demote("too many errors");
    assert!(event.is_some());
    let event = event.unwrap();
    assert_eq!(event.from_tier, "autonomous");
    assert_eq!(event.to_tier, "supervised");
    assert_eq!(engine.current_tier(), "supervised");
}

#[test]
fn demote_at_lowest_returns_none() {
    let def = make_policy(vec![
        ("supervised", Some(("accuracy", 95.0))),
        ("autonomous", None),
    ]);
    let mut engine = PolicyEngine::from_def(&def);
    assert!(engine.demote("reason").is_none());
}

#[test]
fn three_tiers_progressive() {
    let def = make_policy(vec![
        ("restricted", Some(("accuracy", 80.0))),
        ("supervised", Some(("accuracy", 95.0))),
        ("autonomous", None),
    ]);
    let mut engine = PolicyEngine::from_def(&def);
    assert_eq!(engine.tier_count(), 3);
    assert_eq!(engine.current_tier(), "restricted");

    // Promote to supervised.
    let metrics = vec![("accuracy".to_string(), 85.0)];
    engine.evaluate_promotion(&metrics);
    assert_eq!(engine.current_tier(), "supervised");

    // Not enough for autonomous.
    let metrics = vec![("accuracy".to_string(), 90.0)];
    assert!(engine.evaluate_promotion(&metrics).is_none());

    // Now enough.
    let metrics = vec![("accuracy".to_string(), 96.0)];
    engine.evaluate_promotion(&metrics);
    assert_eq!(engine.current_tier(), "autonomous");
    assert!(engine.is_max_tier());
}

#[test]
fn wrong_metric_does_not_promote() {
    let def = make_policy(vec![
        ("supervised", Some(("accuracy", 95.0))),
        ("autonomous", None),
    ]);
    let mut engine = PolicyEngine::from_def(&def);
    let metrics = vec![("latency".to_string(), 99.0)];
    assert!(engine.evaluate_promotion(&metrics).is_none());
}
