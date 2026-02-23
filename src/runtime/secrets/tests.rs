#![allow(clippy::undocumented_unsafe_blocks)]

use serial_test::serial;

use super::*;
use crate::ast::{SecretBinding, SecretSource, SecretsDef, Span};

fn span() -> Span {
    Span { start: 0, end: 0 }
}

#[test]
#[serial]
fn resolves_env_secret() {
    unsafe { std::env::set_var("TEST_REIN_SECRET", "my-api-key") };
    let def = SecretsDef {
        bindings: vec![SecretBinding {
            name: "api_key".to_string(),
            source: SecretSource::Env {
                var: "TEST_REIN_SECRET".to_string(),
            },
            span: span(),
        }],
        span: span(),
    };
    let resolver = SecretResolver::from_def(&def);
    let resolved = resolver.resolve_all().unwrap();
    assert_eq!(resolved["api_key"].value, "my-api-key");
    assert_eq!(resolved["api_key"].source, "env(TEST_REIN_SECRET)");
    unsafe { std::env::remove_var("TEST_REIN_SECRET") };
}

#[test]
#[serial]
fn env_not_found_returns_error() {
    unsafe { std::env::remove_var("NONEXISTENT_VAR_REIN_TEST") };
    let def = SecretsDef {
        bindings: vec![SecretBinding {
            name: "missing".to_string(),
            source: SecretSource::Env {
                var: "NONEXISTENT_VAR_REIN_TEST".to_string(),
            },
            span: span(),
        }],
        span: span(),
    };
    let resolver = SecretResolver::from_def(&def);
    let result = resolver.resolve_all();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        SecretError::EnvNotFound {
            binding: "missing".to_string(),
            var: "NONEXISTENT_VAR_REIN_TEST".to_string(),
        }
    );
}

#[test]
#[serial]
fn vault_falls_back_to_env() {
    unsafe { std::env::set_var("VAULT_SECRET_REIN_KEY", "vault-value") };
    let def = SecretsDef {
        bindings: vec![SecretBinding {
            name: "db_pass".to_string(),
            source: SecretSource::Vault {
                path: "secret/rein/key".to_string(),
            },
            span: span(),
        }],
        span: span(),
    };
    let resolver = SecretResolver::from_def(&def);
    let resolved = resolver.resolve_all().unwrap();
    // Cleanup before assert so env var is removed even if assertion panics.
    unsafe { std::env::remove_var("VAULT_SECRET_REIN_KEY") };
    assert_eq!(resolved["db_pass"].value, "vault-value");
    assert_eq!(
        resolved["db_pass"].source,
        "vault(secret/rein/key)->env(VAULT_SECRET_REIN_KEY)"
    );
}

#[test]
#[serial]
fn vault_unavailable_without_env() {
    unsafe { std::env::remove_var("VAULT_SECRET_MISSING_PATH") };
    let def = SecretsDef {
        bindings: vec![SecretBinding {
            name: "missing".to_string(),
            source: SecretSource::Vault {
                path: "secret/missing-path".to_string(),
            },
            span: span(),
        }],
        span: span(),
    };
    let resolver = SecretResolver::from_def(&def);
    assert!(resolver.resolve_all().is_err());
}

#[test]
#[serial]
fn resolve_single_by_name() {
    unsafe { std::env::set_var("TEST_REIN_SINGLE", "single-val") };
    let def = SecretsDef {
        bindings: vec![SecretBinding {
            name: "token".to_string(),
            source: SecretSource::Env {
                var: "TEST_REIN_SINGLE".to_string(),
            },
            span: span(),
        }],
        span: span(),
    };
    let resolver = SecretResolver::from_def(&def);
    let secret = resolver.resolve("token").unwrap();
    assert_eq!(secret.value, "single-val");
    unsafe { std::env::remove_var("TEST_REIN_SINGLE") };
}

#[test]
fn count_reports_bindings() {
    let def = SecretsDef {
        bindings: vec![
            SecretBinding {
                name: "a".to_string(),
                source: SecretSource::Env {
                    var: "A".to_string(),
                },
                span: span(),
            },
            SecretBinding {
                name: "b".to_string(),
                source: SecretSource::Env {
                    var: "B".to_string(),
                },
                span: span(),
            },
        ],
        span: span(),
    };
    let resolver = SecretResolver::from_def(&def);
    assert_eq!(resolver.count(), 2);
}

// #357: Vault fallback to env var must warn explicitly and mark source as env-based.
#[test]
#[serial]
fn vault_fallback_source_indicates_env_fallback() {
    unsafe { std::env::set_var("VAULT_SECRET_REIN_KEY", "vault-value") };
    let def = SecretsDef {
        bindings: vec![SecretBinding {
            name: "db_pass".to_string(),
            source: SecretSource::Vault {
                path: "secret/rein/key".to_string(),
            },
            span: span(),
        }],
        span: span(),
    };
    let resolver = SecretResolver::from_def(&def);
    let resolved = resolver.resolve_all().unwrap();
    // Cleanup before assert so env var is removed even if assertion panics.
    unsafe { std::env::remove_var("VAULT_SECRET_REIN_KEY") };
    // The source field must encode the composite vault(path)->env(KEY) format
    // so operators can reconstruct exactly which vault path was attempted and
    // which env var was used as fallback.
    assert_eq!(
        resolved["db_pass"].source, "vault(secret/rein/key)->env(VAULT_SECRET_REIN_KEY)",
        "source must be composite vault->env format"
    );
}

#[test]
fn resolve_unregistered_binding_returns_binding_not_found() {
    let def = SecretsDef {
        bindings: vec![SecretBinding {
            name: "token".to_string(),
            source: SecretSource::Env {
                var: "SOME_VAR".to_string(),
            },
            span: span(),
        }],
        span: span(),
    };
    let resolver = SecretResolver::from_def(&def);
    let err = resolver.resolve("nonexistent").unwrap_err();
    assert_eq!(err, SecretError::BindingNotFound("nonexistent".to_string()));
}

#[test]
#[serial]
fn vault_path_with_non_alphanumeric_chars_maps_to_valid_env_key() {
    // Paths with dots, @, and other non-ASCII-alphanumeric chars must all become '_'.
    // "secret/my.service@v2" → VAULT_SECRET_MY_SERVICE_V2
    let expected_env_key = "VAULT_SECRET_MY_SERVICE_V2";
    unsafe { std::env::set_var(expected_env_key, "svcval") };
    let def = SecretsDef {
        bindings: vec![SecretBinding {
            name: "svc".to_string(),
            source: SecretSource::Vault {
                path: "secret/my.service@v2".to_string(),
            },
            span: span(),
        }],
        span: span(),
    };
    let resolver = SecretResolver::from_def(&def);
    let resolved = resolver.resolve_all().unwrap();
    assert_eq!(resolved["svc"].value, "svcval");
    unsafe { std::env::remove_var(expected_env_key) };
}
