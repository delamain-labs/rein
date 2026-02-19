use super::*;

fn test_config() -> ProviderConfig {
    ProviderConfig {
        openai_api_key: Some("test-key".to_string()),
        openai_base_url: None,
    }
}

#[test]
fn resolve_openai_bare() {
    let provider = resolve("openai", &test_config()).expect("should resolve");
    assert_eq!(provider.name(), "openai");
}

#[test]
fn resolve_anthropic_bare() {
    let provider = resolve("anthropic", &test_config()).expect("should resolve");
    assert_eq!(provider.name(), "openai");
}

#[test]
fn resolve_gpt4o_bare() {
    let provider = resolve("gpt-4o", &test_config()).expect("should resolve");
    assert_eq!(provider.name(), "openai");
}

#[test]
fn resolve_with_specific_model() {
    let provider = resolve("openai/gpt-4o-mini", &test_config()).expect("should resolve");
    assert_eq!(provider.name(), "openai");
}

#[test]
fn resolve_case_insensitive() {
    let provider = resolve("OpenAI", &test_config()).expect("should resolve");
    assert_eq!(provider.name(), "openai");
}

#[test]
fn resolve_unknown_provider() {
    let err = resolve("llama-local", &test_config()).err().expect("should fail");
    assert_eq!(err, ResolveError::UnknownProvider("llama-local".to_string()));
    assert!(err.to_string().contains("unknown provider"));
}

#[test]
fn resolve_missing_api_key() {
    let config = ProviderConfig {
        openai_api_key: None,
        openai_base_url: None,
    };
    let err = resolve("openai", &config).err().expect("should fail");
    assert_eq!(err, ResolveError::MissingApiKey("openai".to_string()));
    assert!(err.to_string().contains("missing API key"));
}

#[test]
fn resolve_with_custom_base_url() {
    let config = ProviderConfig {
        openai_api_key: Some("key".to_string()),
        openai_base_url: Some("http://localhost:8080".to_string()),
    };
    let provider = resolve("openai", &config).expect("should resolve");
    assert_eq!(provider.name(), "openai");
}
