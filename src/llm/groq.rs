//! Groq model registry and the shared chat [`Message`] type.
//!
//! The live chat client now lives in [`crate::llm::provider`] (Groq is served by
//! `OpenAiCompatClient`). This module retains two things every provider depends
//! on: the [`Message`] wire type (re-exported as `crate::llm::Message`) and the
//! Groq [`GroqClient::MODELS`] table, consumed by `provider::suggested_models`
//! and the TUI Config pane.

use serde::{Deserialize, Serialize};

/// A single chat message (`role` = `system` / `user` / `assistant`).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// Namespace for the Groq model registry.
///
/// Retained as a zero-sized marker so existing references to
/// `GroqClient::MODELS` keep compiling; the actual client is
/// [`crate::llm::provider::OpenAiCompatClient`].
pub struct GroqClient;

impl GroqClient {
    /// Available models on Groq: `(id, description, context_window_tokens)`.
    pub const MODELS: &'static [(&'static str, &'static str, usize)] = &[
        (
            "openai/gpt-oss-120b",
            "GPT-OSS 120B - Most powerful",
            131072,
        ),
        (
            "llama-3.3-70b-versatile",
            "Llama 3.3 70B - Best for complex tasks",
            131072,
        ),
        (
            "llama-3.1-8b-instant",
            "Llama 3.1 8B - Fast and efficient",
            131072,
        ),
        ("mixtral-8x7b-32768", "Mixtral 8x7B - Good balance", 32768),
        ("gemma2-9b-it", "Gemma 2 9B - Google's model", 8192),
    ];
}
