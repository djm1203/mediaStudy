use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub groq_api_key: Option<String>,
    pub default_model: Option<String>,
    pub data_dir: Option<PathBuf>,
    pub current_bucket: Option<String>,
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

    /// Check if Groq API key is configured
    pub fn has_api_key(&self) -> bool {
        self.groq_api_key.as_ref().is_some_and(|k| !k.is_empty())
            || std::env::var("GROQ_API_KEY").is_ok()
    }

    /// Get the Groq API key, checking environment variable as fallback
    pub fn get_api_key(&self) -> Option<String> {
        self.groq_api_key
            .clone()
            .filter(|k| !k.is_empty())
            .or_else(|| std::env::var("GROQ_API_KEY").ok())
    }
}
