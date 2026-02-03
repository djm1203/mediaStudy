use anyhow::{Context, Result};
use std::path::Path;

/// Extract text content from a text/markdown file
pub fn extract(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("Failed to read text file: {:?}", path))
}
