use super::*;

fn openai_config() -> ProviderConfig {
    ProviderConfig {
        openai_api_key: Some("test-key".to_string()),
        ..ProviderConfig::default()
    }
}

fn anthropic_config() -> ProviderConfig {
    ProviderConfig {
        anthropic_api_key: Some("test-key".to_string()),
        ..ProviderConfig::default()
    }
}

fn full_config() -> ProviderConfig {
    ProviderConfig {
        openai_api_key: Some("oai-key".to_string()),
        anthropic_api_key: Some("ant-key".to_string()),
        ..ProviderConfig::default()
    }
}

#[test]
fn resolve_openai_bare() {
    let provider = resolve("openai", &openai_config()).expect("should resolve");
    assert_eq!(provider.name(), "openai");
}

#[test]
fn resolve_anthropic_bare() {
    let provider = resolve("anthropic", &anthropic_config()).expect("should resolve");
    assert_eq!(provider.name(), "anthropic");
}

#[test]
fn resolve_claude_bare() {
    let provider = resolve("claude", &anthropic_config()).expect("should resolve");
    assert_eq!(provider.name(), "anthropic");
}

#[test]
fn resolve_gpt4o_bare() {
    let provider = resolve("gpt-4o", &openai_config()).expect("should resolve");
    assert_eq!(provider.name(), "openai");
}

#[test]
fn resolve_with_specific_model() {
    let provider = resolve("openai/gpt-4o-mini", &openai_config()).expect("should resolve");
    assert_eq!(provider.name(), "openai");
}

#[test]
fn resolve_anthropic_specific_model() {
    let provider = resolve("anthropic/claude-opus-4-20250514", &anthropic_config()).expect("should resolve");
    assert_eq!(provider.name(), "anthropic");
}

#[test]
fn resolve_case_insensitive() {
    let provider = resolve("OpenAI", &openai_config()).expect("should resolve");
    assert_eq!(provider.name(), "openai");
}

#[test]
fn resolve_unknown_provider() {
    let err = resolve("llama-local", &full_config()).err().expect("should fail");
    assert_eq!(err, ResolveError::UnknownProvider("llama-local".to_string()));
}

#[test]
fn resolve_missing_openai_key() {
    let config = ProviderConfig::default();
    let err = resolve("openai", &config).err().expect("should fail");
    assert_eq!(err, ResolveError::MissingApiKey("openai".to_string()));
}

#[test]
fn resolve_missing_anthropic_key() {
    let config = ProviderConfig::default();
    let err = resolve("anthropic", &config).err().expect("should fail");
    assert_eq!(err, ResolveError::MissingApiKey("anthropic".to_string()));
}

#[test]
fn resolve_with_custom_base_url() {
    let config = ProviderConfig {
        openai_api_key: Some("key".to_string()),
        openai_base_url: Some("http://localhost:8080".to_string()),
        ..ProviderConfig::default()
    };
    let provider = resolve("openai", &config).expect("should resolve");
    assert_eq!(provider.name(), "openai");
}

#[test]
fn resolve_anthropic_with_custom_base_url() {
    let config = ProviderConfig {
        anthropic_api_key: Some("key".to_string()),
        anthropic_base_url: Some("http://localhost:9090".to_string()),
        ..ProviderConfig::default()
    };
    let provider = resolve("anthropic", &config).expect("should resolve");
    assert_eq!(provider.name(), "anthropic");
}
