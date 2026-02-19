use super::openai::OpenAiProvider;
use super::Provider;

#[cfg(test)]
mod tests;

/// Known provider prefixes and their default models.
const PROVIDER_DEFAULTS: &[(&str, &str, &str)] = &[
    // (prefix, base_url, default_model)
    ("openai", "https://api.openai.com/v1", "gpt-4o"),
    ("anthropic", "https://api.openai.com/v1", "claude-sonnet-4-20250514"),
    ("gpt-4o", "https://api.openai.com/v1", "gpt-4o"),
    ("gpt-4", "https://api.openai.com/v1", "gpt-4"),
    ("gpt-3.5", "https://api.openai.com/v1", "gpt-3.5-turbo"),
];

/// Configuration for resolving model fields to providers.
#[derive(Debug, Clone, Default)]
pub struct ProviderConfig {
    pub openai_api_key: Option<String>,
    pub openai_base_url: Option<String>,
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

/// Resolve a `.rein` model field (e.g. `"openai"`, `"gpt-4o"`, `"anthropic"`)
/// into a boxed `Provider`.
///
/// # Errors
/// Returns `ResolveError` if the model is unknown or the API key is missing.
pub fn resolve(
    model_field: &str,
    config: &ProviderConfig,
) -> Result<Box<dyn Provider>, ResolveError> {
    let normalized = model_field.to_lowercase();

    // Check for exact or prefix match
    let (_prefix, base_url, model_name) = PROVIDER_DEFAULTS
        .iter()
        .find(|(prefix, _, _)| normalized == *prefix || normalized.starts_with(&format!("{prefix}/")))
        .ok_or_else(|| ResolveError::UnknownProvider(model_field.to_string()))?;

    // If the model field contains a slash, treat the part after as the specific model
    let actual_model = if let Some(idx) = model_field.find('/') {
        &model_field[idx + 1..]
    } else {
        model_name
    };

    // All known providers currently use the OpenAI-compatible API
    let api_key = config
        .openai_api_key
        .as_deref()
        .ok_or_else(|| ResolveError::MissingApiKey("openai".to_string()))?;

    let url = config
        .openai_base_url
        .clone()
        .unwrap_or_else(|| (*base_url).to_string());

    Ok(Box::new(OpenAiProvider::new(api_key, actual_model, Some(url))))
}
