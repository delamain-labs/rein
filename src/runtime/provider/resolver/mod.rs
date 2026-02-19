use super::Provider;
use super::anthropic::AnthropicProvider;
use super::openai::OpenAiProvider;

#[cfg(test)]
mod tests;

/// Configuration for resolving model fields to providers.
#[derive(Debug, Clone, Default)]
pub struct ProviderConfig {
    pub openai_api_key: Option<String>,
    pub openai_base_url: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub anthropic_base_url: Option<String>,
}

/// Errors from model resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// The model field doesn't match any known provider.
    UnknownProvider(String),
    /// The required API key is missing.
    MissingApiKey(String),
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownProvider(model) => write!(f, "unknown provider for model: {model}"),
            Self::MissingApiKey(provider) => {
                write!(f, "missing API key for provider: {provider}")
            }
        }
    }
}

impl std::error::Error for ResolveError {}

/// Which backend a model field maps to.
enum Backend {
    OpenAi,
    Anthropic,
}

/// Known model prefixes.
struct ModelMapping {
    prefix: &'static str,
    backend: Backend,
    default_model: &'static str,
}

const MAPPINGS: &[ModelMapping] = &[
    ModelMapping {
        prefix: "openai",
        backend: Backend::OpenAi,
        default_model: "gpt-4o",
    },
    ModelMapping {
        prefix: "gpt-4o-mini",
        backend: Backend::OpenAi,
        default_model: "gpt-4o-mini",
    },
    ModelMapping {
        prefix: "gpt-4o",
        backend: Backend::OpenAi,
        default_model: "gpt-4o",
    },
    ModelMapping {
        prefix: "gpt-4",
        backend: Backend::OpenAi,
        default_model: "gpt-4",
    },
    ModelMapping {
        prefix: "gpt-3.5",
        backend: Backend::OpenAi,
        default_model: "gpt-3.5-turbo",
    },
    ModelMapping {
        prefix: "anthropic",
        backend: Backend::Anthropic,
        default_model: "claude-sonnet-4-20250514",
    },
    ModelMapping {
        prefix: "claude",
        backend: Backend::Anthropic,
        default_model: "claude-sonnet-4-20250514",
    },
];

/// Resolve a `.rein` model field into a boxed `Provider`.
///
/// # Errors
/// Returns `ResolveError` if the model is unknown or the API key is missing.
pub fn resolve(
    model_field: &str,
    config: &ProviderConfig,
) -> Result<Box<dyn Provider>, ResolveError> {
    let normalized = model_field.to_lowercase();

    let mapping = MAPPINGS
        .iter()
        .find(|m| normalized == m.prefix || normalized.starts_with(&format!("{}/", m.prefix)))
        .ok_or_else(|| ResolveError::UnknownProvider(model_field.to_string()))?;

    let actual_model = model_field
        .find('/')
        .map_or(mapping.default_model, |i| &model_field[i + 1..]);

    match mapping.backend {
        Backend::OpenAi => {
            let api_key = config
                .openai_api_key
                .as_deref()
                .ok_or_else(|| ResolveError::MissingApiKey("openai".to_string()))?;
            let url = config.openai_base_url.clone();
            Ok(Box::new(OpenAiProvider::new(api_key, actual_model, url)))
        }
        Backend::Anthropic => {
            let api_key = config
                .anthropic_api_key
                .as_deref()
                .ok_or_else(|| ResolveError::MissingApiKey("anthropic".to_string()))?;
            let url = config.anthropic_base_url.clone();
            Ok(Box::new(AnthropicProvider::new(
                api_key,
                actual_model,
                url,
                None,
            )))
        }
    }
}
