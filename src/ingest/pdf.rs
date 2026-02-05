use anyhow::{Context, Result};
use std::panic;
use std::path::Path;

/// Extract text content from a PDF file
pub fn extract(path: &Path) -> Result<String> {
    let bytes =
        std::fs::read(path).with_context(|| format!("Failed to read PDF file: {:?}", path))?;

    // Try pdf_extract first, but catch panics (it can crash on complex PDFs)
    let extract_result = panic::catch_unwind(|| pdf_extract::extract_text_from_mem(&bytes));

    let text = match extract_result {
        Ok(Ok(text)) => text,
        Ok(Err(e)) => {
            // pdf_extract returned an error, try fallback
            eprintln!("Warning: pdf_extract failed, trying fallback: {}", e);
            extract_with_lopdf(&bytes)?
        }
        Err(_) => {
            // pdf_extract panicked, try fallback
            eprintln!("Warning: pdf_extract crashed, trying fallback extraction");
            extract_with_lopdf(&bytes)?
        }
    };

    // Clean up the extracted text
    let cleaned = text
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    if cleaned.is_empty() {
        anyhow::bail!("No text could be extracted from PDF: {:?}", path);
    }

    Ok(cleaned)
}

/// Fallback PDF text extraction using lopdf
fn extract_with_lopdf(bytes: &[u8]) -> Result<String> {
    use lopdf::Document;

    let doc = Document::load_mem(bytes).context("Failed to load PDF with lopdf")?;

    let mut text = String::new();

    let pages = doc.get_pages();
    for (page_num, _) in pages {
        if let Ok(page_text) = doc.extract_text(&[page_num]) {
            text.push_str(&page_text);
            text.push('\n');
        }
    }

    if text.trim().is_empty() {
        anyhow::bail!("Could not extract any text from PDF (may be scanned/image-based)");
    }

    Ok(text)
}
