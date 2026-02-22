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
        let _ = dotenvy::dotenv();
        let toml_config = Self::load_toml(dir);
        Self::from_toml(toml_config)
    }

    /// Load config for a specific environment (e.g. "dev", "staging", "production").
    /// Merges `rein.toml` base with `rein.env.<env>.toml` overrides.
    pub fn load_for_env(dir: &Path, env_name: &str) -> Self {
        let _ = dotenvy::dotenv();

        let mut toml_config = Self::load_toml(dir);
        let env_path = dir.join(format!("rein.env.{env_name}.toml"));
        if let Ok(content) = std::fs::read_to_string(&env_path)
            && let Ok(overrides) = toml::from_str::<ReinToml>(&content)
        {
            Self::merge_toml(&mut toml_config, &overrides);
        }

        Self::from_toml(toml_config)
    }

    fn load_toml(dir: &Path) -> ReinToml {
        let path = dir.join("rein.toml");
        match std::fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => ReinToml::default(),
        }
    }

    /// Merge overrides into base config. Non-default values from overrides win.
    fn merge_toml(base: &mut ReinToml, overrides: &ReinToml) {
        if !overrides.project.name.is_empty() {
            base.project.name.clone_from(&overrides.project.name);
        }
        if overrides.runtime.default_model.is_some() {
            base.runtime.default_model.clone_from(&overrides.runtime.default_model);
        }
        if overrides.runtime.timeout_secs.is_some() {
            base.runtime.timeout_secs = overrides.runtime.timeout_secs;
        }
        if !overrides.runtime.log_level.is_empty() && overrides.runtime.log_level != "info" {
            base.runtime.log_level.clone_from(&overrides.runtime.log_level);
        }
        if overrides.runtime.max_retries != 3 {
            base.runtime.max_retries = overrides.runtime.max_retries;
        }
        if overrides.observability.trace_output.is_some() {
            base.observability.trace_output.clone_from(&overrides.observability.trace_output);
        }
        if overrides.deploy.target.is_some() {
            base.deploy.target.clone_from(&overrides.deploy.target);
        }
        if overrides.deploy.region.is_some() {
            base.deploy.region.clone_from(&overrides.deploy.region);
        }
    }

    fn from_toml(toml_config: ReinToml) -> Self {
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
