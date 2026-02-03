use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::config::Config;

/// Represents a knowledge bucket (isolated dataset)
#[derive(Debug, Clone)]
pub struct Bucket {
    pub name: String,
    pub path: PathBuf,
}

impl Bucket {
    /// Get the buckets directory
    pub fn buckets_dir() -> Result<PathBuf> {
        Ok(Config::data_dir()?.join("buckets"))
    }

    /// Create a new bucket
    pub fn create(name: &str) -> Result<Self> {
        let name = Self::sanitize_name(name);
        let path = Self::buckets_dir()?.join(&name);

        if path.exists() {
            anyhow::bail!("Bucket '{}' already exists", name);
        }

        std::fs::create_dir_all(&path)
            .with_context(|| format!("Failed to create bucket directory: {:?}", path))?;

        Ok(Self { name, path })
    }

    /// Open an existing bucket
    pub fn open(name: &str) -> Result<Self> {
        let name = Self::sanitize_name(name);
        let path = Self::buckets_dir()?.join(&name);

        if !path.exists() {
            anyhow::bail!("Bucket '{}' does not exist", name);
        }

        Ok(Self { name, path })
    }

    /// List all buckets
    pub fn list_all() -> Result<Vec<String>> {
        let buckets_dir = Self::buckets_dir()?;

        if !buckets_dir.exists() {
            return Ok(Vec::new());
        }

        let mut buckets = Vec::new();

        for entry in std::fs::read_dir(&buckets_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                if let Some(name) = path.file_name() {
                    buckets.push(name.to_string_lossy().to_string());
                }
            }
        }

        buckets.sort();
        Ok(buckets)
    }

    /// Delete a bucket
    pub fn delete(name: &str) -> Result<()> {
        let bucket = Self::open(name)?;

        std::fs::remove_dir_all(&bucket.path)
            .with_context(|| format!("Failed to delete bucket: {}", name))?;

        Ok(())
    }

    /// Check if a bucket exists
    pub fn exists(name: &str) -> Result<bool> {
        let name = Self::sanitize_name(name);
        let path = Self::buckets_dir()?.join(&name);
        Ok(path.exists())
    }

    /// Get the database path for this bucket
    pub fn db_path(&self) -> PathBuf {
        self.path.join("documents.db")
    }

    /// Sanitize bucket name (lowercase, replace spaces with dashes)
    fn sanitize_name(name: &str) -> String {
        name.trim()
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect()
    }
}

/// Get the current active bucket from config
pub fn get_current_bucket() -> Result<Option<Bucket>> {
    let config = Config::load()?;

    match &config.current_bucket {
        Some(name) => {
            if Bucket::exists(name)? {
                Ok(Some(Bucket::open(name)?))
            } else {
                // Bucket was deleted, clear it
                let mut new_config = Config::load()?;
                new_config.current_bucket = None;
                new_config.save()?;
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

/// Set the current active bucket
pub fn set_current_bucket(name: Option<&str>) -> Result<()> {
    let mut config = Config::load()?;
    config.current_bucket = name.map(|n| Bucket::sanitize_name(n));
    config.save()?;
    Ok(())
}
