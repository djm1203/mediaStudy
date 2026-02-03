use anyhow::{Context, Result};
use std::path::Path;
use tokio::process::Command;

/// Supported image formats for OCR
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "tiff", "tif", "webp"];

/// Check if a file is an image that can be OCR'd
pub fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Validate a file path for safe use with external commands
fn validate_path(path: &Path) -> Result<std::path::PathBuf> {
    // Ensure path exists
    if !path.exists() {
        anyhow::bail!("File does not exist: {:?}", path);
    }

    // Get canonical path to prevent traversal attacks
    let canonical = std::fs::canonicalize(path)
        .with_context(|| format!("Failed to resolve path: {:?}", path))?;

    // Ensure it's a regular file
    if !canonical.is_file() {
        anyhow::bail!("Path is not a regular file: {:?}", path);
    }

    // Verify valid UTF-8 (required for command args)
    if canonical.to_str().is_none() {
        anyhow::bail!("Path contains invalid UTF-8 characters: {:?}", path);
    }

    Ok(canonical)
}

/// Extract text from an image using Tesseract OCR
pub async fn extract_text(path: &Path) -> Result<String> {
    // Validate input path
    let canonical_path = validate_path(path)?;
    let path_str = canonical_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 in image path"))?;

    // Check if tesseract is available
    let check = Command::new("tesseract")
        .arg("--version")
        .output()
        .await;

    if check.is_err() {
        anyhow::bail!(
            "Tesseract OCR not found. Install it with:\n  \
             - Ubuntu/Debian: sudo apt install tesseract-ocr\n  \
             - macOS: brew install tesseract\n  \
             - Windows: https://github.com/UB-Mannheim/tesseract/wiki"
        );
    }

    // Run tesseract with validated path
    let output = Command::new("tesseract")
        .arg(path_str)
        .arg("stdout") // Output to stdout
        .arg("-l")
        .arg("eng") // English language
        .arg("--psm")
        .arg("1") // Automatic page segmentation with OSD
        .output()
        .await
        .context("Failed to run tesseract")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Tesseract failed: {}", stderr);
    }

    let text = String::from_utf8_lossy(&output.stdout).to_string();
    let text = clean_ocr_text(&text);

    if text.is_empty() {
        anyhow::bail!("No text found in image");
    }

    Ok(text)
}

/// Clean up OCR output
fn clean_ocr_text(text: &str) -> String {
    let mut result = String::new();
    let mut prev_was_newline = false;

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if !prev_was_newline && !result.is_empty() {
                result.push('\n');
                prev_was_newline = true;
            }
            continue;
        }

        // Skip lines that are just noise (single characters, etc.)
        if trimmed.len() < 2 && !trimmed.chars().all(|c| c.is_alphanumeric()) {
            continue;
        }

        if !result.is_empty() && !prev_was_newline {
            result.push('\n');
        }

        result.push_str(trimmed);
        prev_was_newline = false;
    }

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_image_file() {
        assert!(is_image_file(Path::new("test.png")));
        assert!(is_image_file(Path::new("test.jpg")));
        assert!(is_image_file(Path::new("test.JPEG")));
        assert!(!is_image_file(Path::new("test.pdf")));
        assert!(!is_image_file(Path::new("test.txt")));
    }

    #[test]
    fn test_clean_ocr_text() {
        let input = "  Hello World  \n\n\n  This is OCR text  \n | \n More text";
        let output = clean_ocr_text(input);
        assert!(output.contains("Hello World"));
        assert!(output.contains("This is OCR text"));
        assert!(output.contains("More text"));
    }
}
