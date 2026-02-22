use serde::{Deserialize, Serialize};

use super::Span;

/// A secret source expression (e.g. `vault("secret/rein/key")`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SecretSource {
    /// `vault("path")` — `HashiCorp` Vault or similar.
    Vault { path: String },
    /// `env("VAR_NAME")` — environment variable.
    Env { var: String },
}

/// A single secret binding: `key: source`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecretBinding {
    pub name: String,
    pub source: SecretSource,
    pub span: Span,
}

/// A `secrets { key: vault("...") }` block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecretsDef {
    pub bindings: Vec<SecretBinding>,
    pub span: Span,
}
