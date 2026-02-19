use super::*;
use std::time::Duration;

// Note: env::set_var/remove_var are unsafe in Rust 2024 edition because
// they can cause data races in multithreaded programs. In tests, we accept
// this risk since tests are the only place we need to manipulate env vars.

#[test]
fn default_model_is_gpt4o() {
    unsafe { env::remove_var("REIN_DEFAULT_MODEL") };
    let config = Config::load();
    assert_eq!(config.default_model, "gpt-4o");
}

#[test]
fn default_timeout_is_30_seconds() {
    unsafe { env::remove_var("REIN_TIMEOUT_SECS") };
    let config = Config::load();
    assert_eq!(config.request_timeout, Duration::from_secs(30));
}

#[test]
fn has_openai_key_false_when_missing() {
    // This test verifies the helper method, not env loading
    let config = Config {
        openai_api_key: None,
        anthropic_api_key: None,
        default_model: "gpt-4o".to_string(),
        request_timeout: Duration::from_secs(30),
    };
    assert!(!config.has_openai_key());
}

#[test]
fn has_openai_key_false_when_empty() {
    let config = Config {
        openai_api_key: Some(String::new()),
        anthropic_api_key: None,
        default_model: "gpt-4o".to_string(),
        request_timeout: Duration::from_secs(30),
    };
    assert!(!config.has_openai_key());
}

#[test]
fn has_openai_key_true_when_set() {
    let config = Config {
        openai_api_key: Some("sk-test-123".to_string()),
        anthropic_api_key: None,
        default_model: "gpt-4o".to_string(),
        request_timeout: Duration::from_secs(30),
    };
    assert!(config.has_openai_key());
}
