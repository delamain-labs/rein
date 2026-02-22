use super::*;

#[test]
fn record_and_export_json() {
    let c = MetricsCollector::new();
    c.record("rein_step_duration_ms", 150.0, HashMap::new());
    let json = c.export(&ExportFormat::Json);
    assert!(json.contains("rein_step_duration_ms"));
}

#[test]
fn prometheus_export() {
    let c = MetricsCollector::new();
    let mut labels = HashMap::new();
    labels.insert("workflow".to_string(), "support".to_string());
    c.record("rein_workflow_total", 1.0, labels);
    let prom = c.export(&ExportFormat::Prometheus);
    assert!(prom.contains("rein_workflow_total{workflow=\"support\"} 1"));
}

#[test]
fn empty_collector() {
    let c = MetricsCollector::new();
    assert!(c.is_empty());
    assert_eq!(c.len(), 0);
}
