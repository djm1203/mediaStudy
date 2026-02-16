use anyhow::{Context, Result};
use reqwest::multipart;
use serde::Deserialize;
use std::path::Path;

const GROQ_WHISPER_URL: &str = "https://api.groq.com/openai/v1/audio/transcriptions";

#[derive(Debug, Clone)]
pub struct WhisperClient {
    client: reqwest::Client,
    api_key: String,
    pub model: String,
}

#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    text: String,
}

impl WhisperClient {
    /// Available Whisper models on Groq
    #[allow(dead_code)]
    pub const MODELS: &'static [(&'static str, &'static str)] = &[
        (
            "whisper-large-v3-turbo",
            "Whisper Large v3 Turbo - Fast and accurate",
        ),
        ("whisper-large-v3", "Whisper Large v3 - Most accurate"),
    ];

    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model: model.unwrap_or_else(|| "whisper-large-v3-turbo".to_string()),
        }
    }

    /// Transcribe an audio file
    pub async fn transcribe(&self, file_path: &Path) -> Result<String> {
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("audio.mp3")
            .to_string();

        let file_bytes = std::fs::read(file_path)
            .with_context(|| format!("Failed to read audio file: {:?}", file_path))?;

        let file_part = multipart::Part::bytes(file_bytes)
            .file_name(file_name)
            .mime_str(Self::guess_mime_type(file_path))?;

        let form = multipart::Form::new()
            .part("file", file_part)
            .text("model", self.model.clone())
            .text("response_format", "json");

        let response = self
            .client
            .post(GROQ_WHISPER_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await
            .context("Failed to send request to Groq Whisper")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Groq Whisper API error ({}): {}", status, text);
        }

        let transcription: TranscriptionResponse = response
            .json()
            .await
            .context("Failed to parse Whisper response")?;

        Ok(transcription.text)
    }

    fn guess_mime_type(path: &Path) -> &'static str {
        match path.extension().and_then(|e| e.to_str()) {
            Some("mp3") => "audio/mpeg",
            Some("mp4") => "audio/mp4",
            Some("m4a") => "audio/mp4",
            Some("wav") => "audio/wav",
            Some("webm") => "audio/webm",
            Some("ogg") => "audio/ogg",
            Some("flac") => "audio/flac",
            _ => "audio/mpeg", // Default
        }
    }
}

/// Check if ffmpeg is available for video processing
pub async fn check_ffmpeg() -> bool {
    tokio::process::Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .is_ok()
}

/// Validate a file path for safe use with external commands
fn validate_path(path: &Path) -> Result<()> {
    // Ensure path exists
    if !path.exists() {
        anyhow::bail!("File does not exist: {:?}", path);
    }

    // Get canonical path to prevent traversal
    let canonical = std::fs::canonicalize(path)
        .with_context(|| format!("Failed to resolve path: {:?}", path))?;

    // Ensure it's a file, not a directory or special file
    if !canonical.is_file() {
        anyhow::bail!("Path is not a regular file: {:?}", path);
    }

    // Check for valid UTF-8 path (required for command args)
    if canonical.to_str().is_none() {
        anyhow::bail!("Path contains invalid UTF-8 characters: {:?}", path);
    }

    Ok(())
}

/// Extract audio from a video file using ffmpeg
pub async fn extract_audio_from_video(video_path: &Path) -> Result<std::path::PathBuf> {
    // Validate input path
    validate_path(video_path)?;

    if !check_ffmpeg().await {
        anyhow::bail!(
            "ffmpeg is required for video transcription. Install it with:\n\
             - Arch: sudo pacman -S ffmpeg\n\
             - Ubuntu: sudo apt install ffmpeg\n\
             - macOS: brew install ffmpeg"
        );
    }

    // Get canonical path for safety
    let canonical_input = std::fs::canonicalize(video_path)?;
    let input_str = canonical_input
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 in video path"))?;

    // Generate unique output filename
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(std::time::Duration::from_secs(0))
        .as_secs();
    let pid = std::process::id();
    let output_path =
        std::env::temp_dir().join(format!("librarian-audio-{}-{}.mp3", pid, timestamp));
    let output_str = output_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 in output path"))?;

    // Use tokio::process for async execution
    let status = tokio::process::Command::new("ffmpeg")
        .args([
            "-i",
            input_str,
            "-vn", // No video
            "-acodec",
            "libmp3lame", // MP3 codec
            "-ar",
            "16000", // 16kHz sample rate (good for speech)
            "-ac",
            "1",  // Mono
            "-y", // Overwrite
            output_str,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .context("Failed to run ffmpeg")?;

    if !status.success() {
        anyhow::bail!("ffmpeg failed to extract audio from video");
    }

    Ok(output_path)
}

/// Check if a file is an audio file
#[allow(dead_code)]
pub fn is_audio_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("mp3" | "wav" | "m4a" | "ogg" | "flac" | "webm")
    )
}

/// Check if a file is a video file
#[allow(dead_code)]
pub fn is_video_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("mp4" | "mkv" | "avi" | "mov" | "webm" | "flv")
    )
}
