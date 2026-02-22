use serde::Deserialize;
use std::env;
use std::path::Path;
use std::time::Duration;

/// Runtime configuration loaded from environment variables, `.env`, and `rein.toml`.
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
    /// Project-level settings from `rein.toml`.
    pub project: ProjectConfig,
}

/// Deserialized `rein.toml` file.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ReinToml {
    pub project: ProjectSection,
    pub runtime: RuntimeSection,
    pub registry: RegistrySection,
    pub deploy: DeploySection,
    pub observability: ObservabilitySection,
    pub dev: DevSection,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ProjectSection {
    pub name: String,
    pub version: String,
    pub description: String,
}

impl Default for ProjectSection {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: "0.1.0".to_string(),
            description: String::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RuntimeSection {
    pub default_model: Option<String>,
    pub timeout_secs: Option<u64>,
    pub max_retries: u32,
    pub log_level: String,
}

impl Default for RuntimeSection {
    fn default() -> Self {
        Self {
            default_model: None,
            timeout_secs: None,
            max_retries: 3,
            log_level: "info".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct RegistrySection {
    pub url: Option<String>,
    pub token_env: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DeploySection {
    pub target: Option<String>,
    pub region: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ObservabilitySection {
    pub trace_output: Option<String>,
    pub metrics_endpoint: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DevSection {
    pub watch: bool,
    pub hot_reload: bool,
}

/// Resolved project configuration (merged from `rein.toml`).
#[derive(Debug, Clone, Default)]
pub struct ProjectConfig {
    pub name: String,
    pub version: String,
    pub max_retries: u32,
    pub log_level: String,
    pub trace_output: Option<String>,
}

impl Config {
    /// Load configuration from `rein.toml`, environment variables, and `.env`.
    pub fn load() -> Self {
        Self::load_from_dir(Path::new("."))
    }

    /// Load configuration from a specific directory.
    pub fn load_from_dir(dir: &Path) -> Self {
        // Load .env file if present
        let _ = dotenvy::dotenv();

        // Load rein.toml if present
        let toml_config = Self::load_toml(dir);

        let default_model = toml_config
            .runtime
            .default_model
            .clone()
            .or_else(|| env::var("REIN_DEFAULT_MODEL").ok())
            .unwrap_or_else(|| "gpt-4o".to_string());

        let timeout = toml_config
            .runtime
            .timeout_secs
            .or_else(|| {
                env::var("REIN_TIMEOUT_SECS")
                    .ok()
                    .and_then(|v| v.parse().ok())
            })
            .unwrap_or(30);

        Self {
            openai_api_key: env::var("OPENAI_API_KEY").ok(),
            anthropic_api_key: env::var("ANTHROPIC_API_KEY").ok(),
            default_model,
            request_timeout: Duration::from_secs(timeout),
            project: ProjectConfig {
                name: toml_config.project.name,
                version: toml_config.project.version,
                max_retries: toml_config.runtime.max_retries,
                log_level: toml_config.runtime.log_level,
                trace_output: toml_config.observability.trace_output,
            },
        }
    }

    fn load_toml(dir: &Path) -> ReinToml {
        let path = dir.join("rein.toml");
        match std::fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => ReinToml::default(),
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
