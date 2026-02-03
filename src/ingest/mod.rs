pub mod chunker;
pub mod ocr;
pub mod pdf;
pub mod text;
pub mod url;

pub use chunker::{chunk_text, Chunk, ChunkConfig};
pub use url::fetch_url;

use anyhow::Result;
use std::path::Path;

use crate::config::Config;
use crate::llm::whisper::{self, WhisperClient};

/// Supported content types
#[derive(Debug, Clone)]
pub enum ContentType {
    Pdf,
    Text,
    Markdown,
    Audio,
    Video,
    Image,
    Url,
    Unknown,
}

impl ContentType {
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()).as_deref() {
            Some("pdf") => ContentType::Pdf,
            Some("txt") => ContentType::Text,
            Some("md" | "markdown") => ContentType::Markdown,
            Some("mp3" | "wav" | "m4a" | "ogg" | "flac") => ContentType::Audio,
            Some("mp4" | "mkv" | "avi" | "mov" | "webm" | "flv") => ContentType::Video,
            Some("png" | "jpg" | "jpeg" | "gif" | "bmp" | "tiff" | "tif" | "webp") => ContentType::Image,
            _ => ContentType::Unknown,
        }
    }

    pub fn is_media(&self) -> bool {
        matches!(self, ContentType::Audio | ContentType::Video)
    }

    pub fn is_image(&self) -> bool {
        matches!(self, ContentType::Image)
    }
}

/// Extracted content from a file
#[derive(Debug, Clone)]
pub struct ExtractedContent {
    pub source: String,
    pub content_type: ContentType,
    pub text: String,
}

/// Extract text content from a file based on its type (sync, for text-based files)
pub fn extract_from_file(path: &Path) -> Result<ExtractedContent> {
    let content_type = ContentType::from_path(path);

    // For media files, we need async transcription
    if content_type.is_media() || content_type.is_image() {
        anyhow::bail!(
            "Media/image files require async processing. Use extract_from_file_async() instead."
        );
    }

    let text = match content_type {
        ContentType::Pdf => pdf::extract(path)?,
        ContentType::Text | ContentType::Markdown => text::extract(path)?,
        ContentType::Unknown => {
            // Try to read as text anyway
            text::extract(path)?
        }
        ContentType::Audio | ContentType::Video | ContentType::Image => unreachable!(),
        ContentType::Url => unreachable!("URLs should use fetch_url() directly"),
    };

    Ok(ExtractedContent {
        source: path.display().to_string(),
        content_type,
        text,
    })
}

/// Extract text content from a file, including media transcription (async)
pub async fn extract_from_file_async(path: &Path) -> Result<ExtractedContent> {
    let content_type = ContentType::from_path(path);

    let text = match &content_type {
        ContentType::Pdf => pdf::extract(path)?,
        ContentType::Text | ContentType::Markdown => text::extract(path)?,
        ContentType::Audio => transcribe_audio(path).await?,
        ContentType::Video => transcribe_video(path).await?,
        ContentType::Image => ocr::extract_text(path).await?,
        ContentType::Url => unreachable!("URLs should use fetch_url() directly"),
        ContentType::Unknown => {
            // Try to read as text anyway
            text::extract(path)?
        }
    };

    Ok(ExtractedContent {
        source: path.display().to_string(),
        content_type,
        text,
    })
}

/// Transcribe an audio file using Groq Whisper
async fn transcribe_audio(path: &Path) -> Result<String> {
    let config = Config::load()?;
    let api_key = config
        .get_api_key()
        .ok_or_else(|| anyhow::anyhow!("No API key configured for transcription"))?;

    let client = WhisperClient::new(api_key, None);
    client.transcribe(path).await
}

/// Transcribe a video file (extract audio first, then transcribe)
async fn transcribe_video(path: &Path) -> Result<String> {
    // Extract audio from video
    let audio_path = whisper::extract_audio_from_video(path).await?;

    // Transcribe the extracted audio
    let result = transcribe_audio(&audio_path).await;

    // Clean up temp file
    let _ = std::fs::remove_file(&audio_path);

    result
}

/// Check if a file requires transcription
pub fn requires_transcription(path: &Path) -> bool {
    ContentType::from_path(path).is_media()
}

/// Check if a file requires async processing (transcription or OCR)
pub fn requires_async_processing(path: &Path) -> bool {
    let ct = ContentType::from_path(path);
    ct.is_media() || ct.is_image()
}
