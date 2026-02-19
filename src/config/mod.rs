use std::env;
use std::time::Duration;

/// Runtime configuration loaded from environment variables and `.env` files.
#[derive(Debug, Clone)]
pub struct Config {
    /// `OpenAI` API key (from the `OPENAI_API_KEY` env var).
    pub openai_api_key: Option<String>,
    /// `Anthropic` API key (from the `ANTHROPIC_API_KEY` env var).
    pub anthropic_api_key: Option<String>,
    /// Default model to use when .rein file doesn't specify one.
    pub default_model: String,
    /// HTTP request timeout.
    pub request_timeout: Duration,
}

impl Config {
    /// Load configuration from environment variables.
    /// Attempts to load `.env` file first (silently ignores if missing).
    pub fn load() -> Self {
        // Load .env file if present (ignore errors)
        let _ = dotenvy::dotenv();

        Self {
            openai_api_key: env::var("OPENAI_API_KEY").ok(),
            anthropic_api_key: env::var("ANTHROPIC_API_KEY").ok(),
            default_model: env::var("REIN_DEFAULT_MODEL").unwrap_or_else(|_| "gpt-4o".to_string()),
            request_timeout: Duration::from_secs(
                env::var("REIN_TIMEOUT_SECS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(30),
            ),
        }
    }

    /// Check if an `OpenAI` API key is configured.
    pub fn has_openai_key(&self) -> bool {
        self.openai_api_key.as_ref().is_some_and(|k| !k.is_empty())
    }

    /// Check if an `Anthropic` API key is configured.
    pub fn has_anthropic_key(&self) -> bool {
        self.anthropic_api_key
            .as_ref()
            .is_some_and(|k| !k.is_empty())
    }
}

#[cfg(test)]
mod tests;
