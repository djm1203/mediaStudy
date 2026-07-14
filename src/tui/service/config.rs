//! Config pane service (blueprint §4).
//!
//! Loads/saves the multi-provider [`Config`] (E2-core). The pane derives its
//! model lists from `provider::suggested_models`, so this layer only ferries the
//! active selections and the per-provider key-set flags. `Config` file I/O is
//! quick but is still run on a blocking thread for consistency.

use anyhow::Result;
use tokio::task;

use crate::config::Config;
use crate::llm::provider::ProviderKind;
use crate::tui::action::{ConfigData, Message};

/// Default Ollama server root shown when the config does not set one.
const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

/// Load the current configuration snapshot as [`Message::ConfigLoaded`].
pub async fn load_config() -> Result<Message> {
    task::spawn_blocking(|| {
        let config = Config::load().unwrap_or_default();
        let data = ConfigData {
            provider: config.provider_kind().to_string(),
            model: config.resolved_model(),
            ollama_url: config
                .ollama_url
                .clone()
                .filter(|u| !u.is_empty())
                .unwrap_or_else(|| DEFAULT_OLLAMA_URL.to_string()),
            // `provider_api_key` also honours the provider's env var, so
            // env-provided keys correctly count as "set".
            groq_key_set: config.provider_api_key(ProviderKind::Groq).is_some(),
            openai_key_set: config.provider_api_key(ProviderKind::OpenAi).is_some(),
            anthropic_key_set: config.provider_api_key(ProviderKind::Anthropic).is_some(),
        };
        Ok(Message::ConfigLoaded(data))
    })
    .await?
}

/// Persist configuration changes as [`Message::ConfigSaved`].
///
/// Loads the existing [`Config`], sets the active `provider`, overwrites
/// `default_model`/`ollama_url` only where the caller passed `Some`, and writes
/// a non-empty `api_key` into the field matching `provider` (Ollama has no key
/// field, so its `api_key` is ignored). Never logs the key.
pub async fn save_config(
    provider: String,
    api_key: Option<String>,
    model: Option<String>,
    ollama_url: Option<String>,
) -> Result<Message> {
    task::spawn_blocking(move || {
        let mut config = Config::load().unwrap_or_default();
        let kind: ProviderKind = provider.parse().unwrap_or(ProviderKind::Groq);
        config.provider = Some(provider);

        if let Some(model) = model.filter(|m| !m.is_empty()) {
            config.default_model = Some(model);
        }
        if let Some(url) = ollama_url.filter(|u| !u.is_empty()) {
            config.ollama_url = Some(url);
        }
        if let Some(key) = api_key.filter(|k| !k.is_empty()) {
            match kind {
                ProviderKind::Groq => config.groq_api_key = Some(key),
                ProviderKind::OpenAi => config.openai_api_key = Some(key),
                ProviderKind::Anthropic => config.anthropic_api_key = Some(key),
                // Ollama needs no key — ignore.
                ProviderKind::Ollama => {}
            }
        }

        config.save()?;
        Ok(Message::ConfigSaved)
    })
    .await?
}
