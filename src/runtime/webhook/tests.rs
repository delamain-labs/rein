use super::*;

#[test]
fn matches_specific_event() {
    let hook = WebhookConfig::new("h", "https://example.com", vec!["workflow.complete".into()]);
    assert!(hook.matches_event("workflow.complete"));
    assert!(!hook.matches_event("workflow.failed"));
}

#[test]
fn wildcard_matches_all() {
    let hook = WebhookConfig::new("h", "https://example.com", vec!["*".into()]);
    assert!(hook.matches_event("anything"));
}

#[test]
fn inactive_never_matches() {
    let mut hook = WebhookConfig::new("h", "https://example.com", vec!["*".into()]);
    hook.active = false;
    assert!(!hook.matches_event("anything"));
}

#[test]
fn sign_payload() {
    let mut hook = WebhookConfig::new("h", "https://example.com", vec![]);
    hook.secret = Some("mysecret".to_string());
    let sig = hook.sign(b"payload");
    assert!(sig.unwrap().starts_with("sha256="));
}

#[test]
fn registry_matching() {
    let mut reg = WebhookRegistry::new();
    reg.register(WebhookConfig::new("a", "https://a.com", vec!["wf.start".into()]));
    reg.register(WebhookConfig::new("b", "https://b.com", vec!["wf.complete".into()]));
    assert_eq!(reg.matching("wf.start").len(), 1);
    assert_eq!(reg.matching("wf.start")[0].name, "a");
}
