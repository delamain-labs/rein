use super::*;

#[test]
fn threshold_fires() {
    let mut engine = AlertEngine::new();
    engine.add_rule(AlertRule {
        name: "high_latency".to_string(),
        condition: AlertCondition::ThresholdExceeded {
            metric: "step_duration_ms".to_string(),
            threshold: 1000.0,
        },
        severity: Severity::Warning,
        channel: "slack".to_string(),
    });

    engine.evaluate_metric("step_duration_ms", 500.0);
    assert!(engine.fired_alerts().is_empty());

    engine.evaluate_metric("step_duration_ms", 1500.0);
    assert_eq!(engine.fired_alerts().len(), 1);
    assert_eq!(engine.fired_alerts()[0].severity, Severity::Warning);
}

#[test]
fn non_matching_metric_ignored() {
    let mut engine = AlertEngine::new();
    engine.add_rule(AlertRule {
        name: "r".to_string(),
        condition: AlertCondition::ThresholdExceeded {
            metric: "errors".to_string(),
            threshold: 10.0,
        },
        severity: Severity::Critical,
        channel: "pager".to_string(),
    });
    engine.evaluate_metric("latency", 9999.0);
    assert!(engine.fired_alerts().is_empty());
}

#[test]
fn clear_alerts() {
    let mut engine = AlertEngine::new();
    engine.add_rule(AlertRule {
        name: "r".to_string(),
        condition: AlertCondition::ThresholdExceeded {
            metric: "m".to_string(),
            threshold: 0.0,
        },
        severity: Severity::Info,
        channel: "log".to_string(),
    });
    engine.evaluate_metric("m", 1.0);
    assert_eq!(engine.fired_alerts().len(), 1);
    engine.clear();
    assert!(engine.fired_alerts().is_empty());
}
