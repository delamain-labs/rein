use crate::ast::{SecretSource, SecretsDef};
use std::collections::HashMap;

#[cfg(test)]
mod tests;

/// A resolved secret value.
#[derive(Debug, Clone)]
pub struct ResolvedSecret {
    pub name: String,
    pub value: String,
    pub source: String,
    /// Non-fatal diagnostic message to surface to the caller (e.g. CLI layer).
    /// `None` means resolution was clean; `Some(msg)` means resolution
    /// succeeded via a fallback path the caller should warn the user about.
    pub warning: Option<String>,
}

/// Errors from secret resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretError {
    /// Environment variable not found. Carries both the binding name and the env var name
    /// so error messages can distinguish between them (they are often different).
    EnvNotFound { binding: String, var: String },
    /// Binding name not registered in this resolver's configuration.
    BindingNotFound(String),
    /// Vault path not accessible (placeholder for real vault integration).
    VaultUnavailable(String),
}

impl std::fmt::Display for SecretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EnvNotFound { binding, var } => {
                write!(f, "binding '{binding}' requires env var '{var}' (not set)")
            }
            Self::BindingNotFound(name) => {
                write!(
                    f,
                    "secret binding '{name}' is not configured in this resolver"
                )
            }
            Self::VaultUnavailable(path) => {
                write!(
                    f,
                    "vault path '{path}' not accessible (vault integration not configured)"
                )
            }
        }
    }
}

/// Resolves secrets from their configured sources.
pub struct SecretResolver {
    bindings: Vec<(String, SecretSource)>,
}

impl SecretResolver {
    /// Create from a parsed secrets definition.
    #[must_use]
    pub fn from_def(def: &SecretsDef) -> Self {
        Self {
            bindings: def
                .bindings
                .iter()
                .map(|b| (b.name.clone(), b.source.clone()))
                .collect(),
        }
    }

    /// Resolve all secrets. Returns resolved values or the first error.
    ///
    /// # Errors
    /// Returns `SecretError` if any secret cannot be resolved.
    pub fn resolve_all(&self) -> Result<HashMap<String, ResolvedSecret>, SecretError> {
        let mut resolved = HashMap::new();
        for (name, source) in &self.bindings {
            let secret = resolve_source(name, source)?;
            resolved.insert(name.clone(), secret);
        }
        Ok(resolved)
    }

    /// Resolve a single secret by name.
    ///
    /// # Errors
    /// Returns `SecretError` if the secret cannot be resolved.
    pub fn resolve(&self, name: &str) -> Result<ResolvedSecret, SecretError> {
        let (_, source) = self
            .bindings
            .iter()
            .find(|(n, _)| n == name)
            .ok_or_else(|| SecretError::BindingNotFound(name.to_string()))?;
        resolve_source(name, source)
    }

    /// Number of configured secrets.
    #[must_use]
    pub fn count(&self) -> usize {
        self.bindings.len()
    }
}

fn resolve_source(name: &str, source: &SecretSource) -> Result<ResolvedSecret, SecretError> {
    match source {
        SecretSource::Env { var } => {
            let value = std::env::var(var).map_err(|_| SecretError::EnvNotFound {
                binding: name.to_string(),
                var: var.clone(),
            })?;
            Ok(ResolvedSecret {
                name: name.to_string(),
                value,
                source: format!("env({var})"),
                warning: None,
            })
        }
        SecretSource::Vault { path } => {
            // Vault integration is a placeholder. In production, this would
            // call the Vault HTTP API. For now, check for a VAULT_* env var fallback.
            let env_key = format!(
                "VAULT_{}",
                path.chars()
                    .map(|c| if c.is_ascii_alphanumeric() {
                        c.to_ascii_uppercase()
                    } else {
                        '_'
                    })
                    .collect::<String>()
            );
            match std::env::var(&env_key) {
                Ok(value) => {
                    let warn_msg = format!(
                        "vault path '{path}' is not configured - falling back to \
                         env var '{env_key}'. Add real Vault integration or use \
                         `env: {env_key}` explicitly."
                    );
                    Ok(ResolvedSecret {
                        name: name.to_string(),
                        value,
                        source: format!("vault({path})->env({env_key})"),
                        warning: Some(warn_msg),
                    })
                }
                Err(_) => Err(SecretError::VaultUnavailable(path.clone())),
            }
        }
    }
}
