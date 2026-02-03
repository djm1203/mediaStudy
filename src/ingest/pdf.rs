use anyhow::{Context, Result};
use std::path::Path;

/// Extract text content from a PDF file
pub fn extract(path: &Path) -> Result<String> {
    let bytes =
        std::fs::read(path).with_context(|| format!("Failed to read PDF file: {:?}", path))?;

    let text = pdf_extract::extract_text_from_mem(&bytes)
        .with_context(|| format!("Failed to extract text from PDF: {:?}", path))?;

    // Clean up the extracted text
    let cleaned = text
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    Ok(cleaned)
}
