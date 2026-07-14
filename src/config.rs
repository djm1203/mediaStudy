use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::llm::provider::{self, AnthropicClient, OpenAiCompatClient, Provider, ProviderKind};
use crate::llm::whisper::Transcriber;

/// Default Ollama server root (base URL is this + `/v1`).
const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub groq_api_key: Option<String>,
    pub default_model: Option<String>,
    pub data_dir: Option<PathBuf>,
    pub current_bucket: Option<String>,
    /// Selected LLM provider (`groq` / `openai` / `anthropic` / `ollama`).
    /// Unset ⇒ Groq, preserving existing configs.
    pub provider: Option<String>,
    pub openai_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    /// Ollama server root (defaults to `http://localhost:11434`).
    pub ollama_url: Option<String>,
}

impl Config {
    /// Get the config directory path
    pub fn config_dir() -> Result<PathBuf> {
        let base = dirs::config_dir().context("Could not determine config directory")?;
        let new_dir = base.join("librarian");
        let old_dir = base.join("media-study");

        // Migrate from old path if needed
        if !new_dir.exists()
            && old_dir.exists()
            && let Err(e) = std::fs::rename(&old_dir, &new_dir)
        {
            eprintln!("Note: Could not migrate config from {:?}: {}", old_dir, e);
            return Ok(old_dir);
        }

        Ok(new_dir)
    }

    /// Get the config file path
    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    /// Get the data directory path
    pub fn data_dir() -> Result<PathBuf> {
        let base = dirs::data_dir().context("Could not determine data directory")?;
        let new_dir = base.join("librarian");
        let old_dir = base.join("media-study");

        // Migrate from old path if needed
        if !new_dir.exists()
            && old_dir.exists()
            && let Err(e) = std::fs::rename(&old_dir, &new_dir)
        {
            eprintln!("Note: Could not migrate data from {:?}: {}", old_dir, e);
            return Ok(old_dir);
        }

        Ok(new_dir)
    }

    /// Load config from file, or return default if not found
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if path.exists() {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read config from {:?}", path))?;
            let config: Config =
                toml::from_str(&content).with_context(|| "Failed to parse config file")?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    /// Save config to file with secure permissions (600)
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let dir = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Config path has no parent directory"))?;

        std::fs::create_dir_all(dir)
            .with_context(|| format!("Failed to create config directory {:?}", dir))?;

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        std::fs::write(&path, &content)
            .with_context(|| format!("Failed to write config to {:?}", path))?;

        // Set restrictive permissions (owner read/write only) to protect API key
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&path)?.permissions();
            perms.set_mode(0o600); // rw-------
            std::fs::set_permissions(&path, perms)
                .with_context(|| "Failed to set config file permissions")?;
        }

        Ok(())
    }

    /// Check if the *selected* provider has a usable credential.
    ///
    /// Ollama needs no key (always `true`). Groq keeps the original
    /// config-field-or-`GROQ_API_KEY`-env check.
    pub fn has_api_key(&self) -> bool {
        let kind = self.provider_kind();
        match kind {
            ProviderKind::Ollama => true,
            ProviderKind::Groq => {
                self.groq_api_key.as_ref().is_some_and(|k| !k.is_empty())
                    || std::env::var("GROQ_API_KEY").is_ok()
            }
            _ => self.provider_api_key(kind).is_some(),
        }
    }

    /// Get the Groq API key, checking environment variable as fallback.
    /// Retained (per E2-core spec) for callers that want the Groq key directly.
    #[allow(dead_code)]
    pub fn get_api_key(&self) -> Option<String> {
        self.provider_api_key(ProviderKind::Groq)
    }

    /// The configured provider, defaulting to Groq when unset/invalid.
    pub fn provider_kind(&self) -> ProviderKind {
        self.provider
            .as_deref()
            .and_then(|p| p.parse().ok())
            .unwrap_or(ProviderKind::Groq)
    }

    /// The API key for a given provider: config field first, then the
    /// provider's conventional env var. Ollama has no key (`None`).
    pub fn provider_api_key(&self, kind: ProviderKind) -> Option<String> {
        let (field, env) = match kind {
            ProviderKind::Groq => (&self.groq_api_key, "GROQ_API_KEY"),
            ProviderKind::OpenAi => (&self.openai_api_key, "OPENAI_API_KEY"),
            ProviderKind::Anthropic => (&self.anthropic_api_key, "ANTHROPIC_API_KEY"),
            ProviderKind::Ollama => return None,
        };
        field
            .clone()
            .filter(|k| !k.is_empty())
            .or_else(|| std::env::var(env).ok().filter(|k| !k.is_empty()))
    }

    /// The model id that will actually be used: the `default_model` field when
    /// set, else the selected provider's default.
    pub fn resolved_model(&self) -> String {
        self.default_model
            .clone()
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| provider::default_model(self.provider_kind()).to_string())
    }

    /// Build a ready-to-call [`Provider`] for the selected provider.
    pub fn resolve_provider(&self) -> Result<Provider> {
        let kind = self.provider_kind();
        let model = self.resolved_model();
        let ctx_window = provider::ctx_for(kind, &model);

        match kind {
            ProviderKind::Anthropic => {
                let key = self.provider_api_key(kind).ok_or_else(|| {
                    anyhow::anyhow!(
                        "No Anthropic API key configured. Set `anthropic_api_key` in config \
                         or the ANTHROPIC_API_KEY environment variable."
                    )
                })?;
                Ok(Provider::Anthropic(AnthropicClient::new(
                    key, model, ctx_window,
                )))
            }
            ProviderKind::Ollama => {
                let root = self
                    .ollama_url
                    .as_deref()
                    .filter(|u| !u.is_empty())
                    .unwrap_or(DEFAULT_OLLAMA_URL)
                    .trim_end_matches('/');
                let base_url = format!("{root}/v1");
                Ok(Provider::OpenAiCompat(OpenAiCompatClient::new(
                    "Ollama", base_url, None, model, ctx_window,
                )))
            }
            ProviderKind::Groq | ProviderKind::OpenAi => {
                let (label, base_url, env) = match kind {
                    ProviderKind::Groq => {
                        ("Groq", "https://api.groq.com/openai/v1", "GROQ_API_KEY")
                    }
                    _ => ("OpenAI", "https://api.openai.com/v1", "OPENAI_API_KEY"),
                };
                let key = self.provider_api_key(kind).ok_or_else(|| {
                    anyhow::anyhow!(
                        "No {label} API key configured. Set it in config or the {env} \
                         environment variable."
                    )
                })?;
                Ok(Provider::OpenAiCompat(OpenAiCompatClient::new(
                    label,
                    base_url,
                    Some(key),
                    model,
                    ctx_window,
                )))
            }
        }
    }

    /// Build an audio [`Transcriber`], preferring Groq then OpenAI.
    pub fn resolve_transcriber(&self) -> Result<Transcriber> {
        if let Some(key) = self.provider_api_key(ProviderKind::Groq) {
            return Ok(Transcriber::groq(key));
        }
        if let Some(key) = self.provider_api_key(ProviderKind::OpenAi) {
            return Ok(Transcriber::openai(key));
        }
        anyhow::bail!("no transcription provider configured — set a Groq or OpenAI API key")
    }
}
