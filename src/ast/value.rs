use serde::{Deserialize, Serialize};

use super::Span;

/// A value expression used in configuration fields.
///
/// Supports literal strings and function calls like `env("VAR_NAME")`.
///
/// # Serde invariant
///
/// This enum uses `#[serde(untagged)]`, which means deserialization tries
/// variants **in declaration order**. `Literal` must remain first so that
/// a plain string deserializes as `Literal` rather than failing to match
/// `EnvRef`'s structured fields. Adding new variants? Put them **after**
/// `Literal` and ensure they have at least one field that a plain string
/// would not satisfy, so the ordering remains unambiguous.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ValueExpr {
    /// A plain string or identifier value.
    Literal(String),
    /// An environment variable reference: `env("VAR_NAME")` or with fallback `env("VAR_NAME", "default")`.
    EnvRef {
        var_name: String,
        default: Option<String>,
        span: Span,
    },
}

/// Error from resolving a `ValueExpr`.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolveError {
    /// An environment variable was not found.
    EnvVarNotSet(String),
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EnvVarNotSet(var) => write!(f, "environment variable '{var}' is not set"),
        }
    }
}

impl std::error::Error for ResolveError {}

impl ValueExpr {
    /// Resolve to a plain string value using the provided env lookup function.
    ///
    /// For `Literal`, returns the string directly. For `EnvRef`, calls
    /// `env_lookup` with the variable name.
    pub fn resolve_with<F>(&self, env_lookup: F) -> Result<String, ResolveError>
    where
        F: Fn(&str) -> Option<String>,
    {
        match self {
            Self::Literal(s) => Ok(s.clone()),
            Self::EnvRef {
                var_name, default, ..
            } => env_lookup(var_name)
                .or_else(|| default.clone())
                .ok_or_else(|| ResolveError::EnvVarNotSet(var_name.clone())),
        }
    }

    /// Resolve using `std::env::var`. Convenience wrapper around `resolve_with`.
    pub fn resolve(&self) -> Result<String, ResolveError> {
        self.resolve_with(|name| std::env::var(name).ok())
    }

    /// Return the literal string value if this is a `Literal`.
    pub fn as_literal(&self) -> Option<&str> {
        match self {
            Self::Literal(s) => Some(s),
            Self::EnvRef { .. } => None,
        }
    }

    /// Return a display-friendly string for this value.
    /// For `Literal`, returns the string. For `EnvRef`, returns the var name.
    pub fn display_value(&self) -> &str {
        match self {
            Self::Literal(s) => s,
            Self::EnvRef { var_name, .. } => var_name,
        }
    }
}
