#![allow(clippy::undocumented_unsafe_blocks)]

use super::*;
use crate::ast::{SecretBinding, SecretSource, SecretsDef, Span};

fn span() -> Span {
    Span { start: 0, end: 0 }
}

#[test]
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
        SecretError::EnvNotFound("NONEXISTENT_VAR_REIN_TEST".to_string())
    );
}

#[test]
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
    assert_eq!(resolved["db_pass"].value, "vault-value");
    unsafe { std::env::remove_var("VAULT_SECRET_REIN_KEY") };
}

#[test]
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
