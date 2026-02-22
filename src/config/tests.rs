use super::*;
use std::time::Duration;
use tempfile::TempDir;

// Note: env::set_var/remove_var are unsafe in Rust 2024 edition because
// they can cause data races in multithreaded programs. In tests, we accept
// this risk since tests are the only place we need to manipulate env vars.

fn make_config(openai: Option<&str>, anthropic: Option<&str>) -> Config {
    Config {
        openai_api_key: openai.map(String::from),
        anthropic_api_key: anthropic.map(String::from),
        default_model: "gpt-4o".to_string(),
        request_timeout: Duration::from_secs(30),
        project: ProjectConfig::default(),
    }
}

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
    let config = make_config(None, None);
    assert!(!config.has_openai_key());
}

#[test]
fn has_openai_key_false_when_empty() {
    let config = make_config(Some(""), None);
    assert!(!config.has_openai_key());
}

#[test]
fn has_openai_key_true_when_set() {
    let config = make_config(Some("sk-test-123"), None);
    assert!(config.has_openai_key());
}

#[test]
fn load_rein_toml() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("rein.toml"),
        r#"
[project]
name = "my-project"
version = "1.0.0"

[runtime]
default_model = "claude-3"
timeout_secs = 60
max_retries = 5

[observability]
trace_output = "traces/"
"#,
    )
    .unwrap();

    let config = Config::load_from_dir(tmp.path());
    assert_eq!(config.default_model, "claude-3");
    assert_eq!(config.request_timeout, Duration::from_secs(60));
    assert_eq!(config.project.name, "my-project");
    assert_eq!(config.project.version, "1.0.0");
    assert_eq!(config.project.max_retries, 5);
    assert_eq!(config.project.trace_output, Some("traces/".to_string()));
}

#[test]
fn load_missing_rein_toml_uses_defaults() {
    let tmp = TempDir::new().unwrap();
    unsafe { env::remove_var("REIN_DEFAULT_MODEL") };
    unsafe { env::remove_var("REIN_TIMEOUT_SECS") };
    let config = Config::load_from_dir(tmp.path());
    assert_eq!(config.default_model, "gpt-4o");
    assert_eq!(config.project.max_retries, 3);
}

#[test]
fn rein_toml_all_sections_deserialize() {
    let toml_str = r#"
[project]
name = "test"
version = "0.1.0"
description = "A test project"

[runtime]
default_model = "gpt-4o"
timeout_secs = 30
max_retries = 3
log_level = "debug"

[registry]
url = "https://registry.rein.dev"
token_env = "REIN_TOKEN"

[deploy]
target = "aws"
region = "us-east-1"

[observability]
trace_output = "traces/"
metrics_endpoint = "http://localhost:9090"

[dev]
watch = true
hot_reload = true
"#;
    let config: ReinToml = toml::from_str(toml_str).unwrap();
    assert_eq!(config.project.name, "test");
    assert_eq!(
        config.registry.url,
        Some("https://registry.rein.dev".to_string())
    );
    assert_eq!(config.deploy.target, Some("aws".to_string()));
    assert!(config.dev.watch);
    assert!(config.dev.hot_reload);
}

#[test]
fn load_env_override_merges() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("rein.toml"),
        r#"
[project]
name = "base"

[runtime]
default_model = "gpt-4o"
timeout_secs = 30
"#,
    )
    .unwrap();

    std::fs::write(
        tmp.path().join("rein.env.staging.toml"),
        r#"
[runtime]
default_model = "gpt-4o-mini"
timeout_secs = 60

[deploy]
target = "aws"
region = "eu-west-1"
"#,
    )
    .unwrap();

    let config = Config::load_for_env(tmp.path(), "staging");
    assert_eq!(config.default_model, "gpt-4o-mini");
    assert_eq!(config.request_timeout, Duration::from_secs(60));
    assert_eq!(config.project.name, "base"); // not overridden
}

#[test]
fn load_env_override_missing_file_uses_base() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("rein.toml"),
        r#"
[runtime]
default_model = "claude-3"
"#,
    )
    .unwrap();

    let config = Config::load_for_env(tmp.path(), "production");
    assert_eq!(config.default_model, "claude-3");
}
