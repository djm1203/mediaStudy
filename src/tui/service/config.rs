//! Config pane service (blueprint §4).
//!
//! Loads/saves `Config` and enumerates `GroqClient::MODELS`. Loading is real
//! already (cheap, file-only, no DB); saving is a stub for Phase 2a. `Config`
//! file I/O is quick but is still run on a blocking thread for consistency.

use anyhow::Result;
use tokio::task;

use crate::config::Config;
use crate::llm::GroqClient;
use crate::tui::action::{ConfigData, Message, ToastLevel};

const DEFAULT_MODEL: &str = "openai/gpt-oss-120b";

/// Load the current configuration snapshot as [`Message::ConfigLoaded`].
pub async fn load_config() -> Result<Message> {
    task::spawn_blocking(|| {
        let config = Config::load().unwrap_or_default();
        let models = GroqClient::MODELS
            .iter()
            .map(|(id, desc, _)| ((*id).to_string(), (*desc).to_string()))
            .collect();
        let data = ConfigData {
            has_api_key: config.has_api_key(),
            model: config
                .default_model
                .clone()
                .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            models,
        };
        Ok(Message::ConfigLoaded(data))
    })
    .await?
}

/// Persist configuration changes as [`Message::ConfigSaved`].
///
/// TODO(config-pane): `spawn_blocking` → load, apply `api_key`/`model`,
/// `Config::save()` → `Message::ConfigSaved`.
pub async fn save_config(api_key: Option<String>, model: Option<String>) -> Result<Message> {
    let _ = (api_key, model);
    Ok(Message::Toast {
        level: ToastLevel::Warn,
        text: "Saving config not implemented yet".to_string(),
    })
}
