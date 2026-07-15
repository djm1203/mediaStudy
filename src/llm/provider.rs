//! Pluggable LLM provider abstraction (E2-core).
//!
//! Rather than trait objects + `async-trait`, providers are a plain enum with
//! native `async fn` methods (enum dispatch). Two concrete clients back every
//! provider:
//!
//! - [`OpenAiCompatClient`] — any endpoint speaking the OpenAI
//!   `POST {base_url}/chat/completions` shape: **Groq**, **OpenAI**, **Ollama**.
//! - [`AnthropicClient`] — Anthropic's `POST /v1/messages` (different request
//!   shape, headers, and SSE event format).
//!
//! All HTTP goes through [`send_with_retry`], which retries 429 / 5xx / transport
//! errors with fixed exponential backoff (B-005).
#![allow(clippy::collapsible_if)]

use anyhow::{Context, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

use crate::llm::groq::{GroqClient, Message};

/// HTTP request timeout for every provider call.
const REQUEST_TIMEOUT_SECS: u64 = 120;
/// Total attempts (1 initial + retries) before giving up.
const MAX_RETRIES: usize = 3;
/// Response-token budget requested from every chat completion.
const MAX_TOKENS: u32 = 4096;

/// Which upstream provider a [`Provider`] talks to. Parsed from the config's
/// `provider` string (`"groq"`, `"openai"`, `"anthropic"`, `"ollama"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Groq,
    OpenAi,
    Anthropic,
    Ollama,
}

impl FromStr for ProviderKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_lowercase().as_str() {
            "groq" => Ok(Self::Groq),
            "openai" => Ok(Self::OpenAi),
            "anthropic" => Ok(Self::Anthropic),
            "ollama" => Ok(Self::Ollama),
            other => anyhow::bail!(
                "unknown provider '{other}' (expected one of: groq, openai, anthropic, ollama)"
            ),
        }
    }
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Groq => "groq",
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
            Self::Ollama => "ollama",
        })
    }
}

// ---------------------------------------------------------------------------
// Model registry
// ---------------------------------------------------------------------------

/// A few current OpenAI chat models `(id, description, context_window_tokens)`.
const OPENAI_MODELS: &[(&str, &str, usize)] = &[
    ("gpt-4o", "GPT-4o - Flagship multimodal", 128000),
    ("gpt-4o-mini", "GPT-4o mini - Fast and cheap", 128000),
];

/// Current Anthropic chat models `(id, description, context_window_tokens)`.
const ANTHROPIC_MODELS: &[(&str, &str, usize)] = &[
    ("claude-opus-4-8", "Claude Opus 4.8 - Most capable", 200000),
    ("claude-sonnet-5", "Claude Sonnet 5 - Balanced", 200000),
    (
        "claude-haiku-4-5-20251001",
        "Claude Haiku 4.5 - Fast",
        200000,
    ),
];

/// Common local Ollama model ids `(id, description, context_window_tokens)`.
/// Context windows are approximate and user-editable.
const OLLAMA_MODELS: &[(&str, &str, usize)] = &[
    ("llama3.1", "Llama 3.1 (local)", 32000),
    ("qwen2.5", "Qwen 2.5 (local)", 32000),
];

/// Suggested models for a provider as `(id, description, context_window_tokens)`.
pub fn suggested_models(kind: ProviderKind) -> &'static [(&'static str, &'static str, usize)] {
    match kind {
        ProviderKind::Groq => GroqClient::MODELS,
        ProviderKind::OpenAi => OPENAI_MODELS,
        ProviderKind::Anthropic => ANTHROPIC_MODELS,
        ProviderKind::Ollama => OLLAMA_MODELS,
    }
}

/// The default model id for a provider when the config does not name one.
pub fn default_model(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Groq => "openai/gpt-oss-120b",
        ProviderKind::OpenAi => "gpt-4o-mini",
        ProviderKind::Anthropic => "claude-sonnet-5",
        ProviderKind::Ollama => "llama3.1",
    }
}

/// A sane fallback context window when a model id is not in the registry.
fn fallback_ctx(kind: ProviderKind) -> usize {
    match kind {
        ProviderKind::Groq => 8192,
        ProviderKind::OpenAi => 128000,
        ProviderKind::Anthropic => 200000,
        ProviderKind::Ollama => 32000,
    }
}

/// Context window (tokens) for `model` under `kind`: looked up in the registry,
/// falling back to a per-provider default when the id is unknown/custom.
pub fn ctx_for(kind: ProviderKind, model: &str) -> usize {
    suggested_models(kind)
        .iter()
        .find(|(id, _, _)| *id == model)
        .map(|(_, _, ctx)| *ctx)
        .unwrap_or_else(|| fallback_ctx(kind))
}

// ---------------------------------------------------------------------------
// Shared HTTP helpers
// ---------------------------------------------------------------------------

/// Build a `reqwest::Client` with a sane request timeout.
fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .unwrap_or_default()
}

/// Backoff delay before the retry that follows attempt `attempt` (1-based):
/// ~0.5s, 1s, 2s …
fn backoff(attempt: usize) -> Duration {
    Duration::from_millis(500u64 << (attempt - 1))
}

/// Trim an error body to a short, single-line-ish snippet for diagnostics.
fn short_body(body: &str) -> String {
    let body = body.trim();
    if body.chars().count() <= 300 {
        body.to_string()
    } else {
        let mut s: String = body.chars().take(300).collect();
        s.push_str("...");
        s
    }
}

/// Send an HTTP request with retry/backoff (B-005).
///
/// `build` is called once per attempt to produce a fresh `RequestBuilder`.
/// Retries on HTTP 429, 5xx, and transport (connect/timeout/request) errors,
/// up to [`MAX_RETRIES`] attempts with exponential backoff. On success returns
/// the (unconsumed) response so the caller can stream or decode it.
async fn send_with_retry<F>(build: F, label: &str) -> Result<reqwest::Response>
where
    F: Fn() -> reqwest::RequestBuilder,
{
    let mut attempt = 0;
    loop {
        attempt += 1;
        match build().send().await {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    return Ok(resp);
                }
                let retryable = status.as_u16() == 429 || status.is_server_error();
                if retryable && attempt < MAX_RETRIES {
                    tokio::time::sleep(backoff(attempt)).await;
                    continue;
                }
                let body = resp.text().await.unwrap_or_default();
                anyhow::bail!("{label} API error ({}): {}", status, short_body(&body));
            }
            Err(e) => {
                let retryable = e.is_timeout() || e.is_connect() || e.is_request();
                if retryable && attempt < MAX_RETRIES {
                    tokio::time::sleep(backoff(attempt)).await;
                    continue;
                }
                anyhow::bail!("{label} request failed after {MAX_RETRIES} retries: {e}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// OpenAI-compatible client (Groq / OpenAI / Ollama)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: Delta,
}

#[derive(Debug, Deserialize)]
struct Delta {
    content: Option<String>,
}

/// Client for any OpenAI-compatible chat-completions endpoint.
///
/// Covers Groq, OpenAI, and Ollama — all speak `POST {base_url}/chat/completions`
/// and the same SSE streaming format. `api_key` is `None` for Ollama (no auth).
#[derive(Debug, Clone)]
pub struct OpenAiCompatClient {
    client: reqwest::Client,
    label: String,
    base_url: String,
    api_key: Option<String>,
    model: String,
    ctx_window: usize,
}

impl OpenAiCompatClient {
    pub fn new(
        label: impl Into<String>,
        base_url: impl Into<String>,
        api_key: Option<String>,
        model: impl Into<String>,
        ctx_window: usize,
    ) -> Self {
        Self {
            client: http_client(),
            label: label.into(),
            base_url: base_url.into(),
            api_key,
            model: model.into(),
            ctx_window,
        }
    }

    fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }

    /// Attach `Authorization: Bearer` when a key is present (Ollama has none).
    fn with_auth(&self, rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.api_key {
            Some(key) => rb.header("Authorization", format!("Bearer {key}")),
            None => rb,
        }
    }

    async fn chat(&self, messages: &[Message]) -> Result<String> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: messages.to_vec(),
            temperature: Some(0.7),
            max_tokens: Some(MAX_TOKENS),
            stream: false,
        };
        let url = self.chat_url();

        let response = send_with_retry(
            || {
                self.with_auth(
                    self.client
                        .post(url.as_str())
                        .header("Content-Type", "application/json")
                        .json(&request),
                )
            },
            &self.label,
        )
        .await?;

        let chat_response: ChatResponse = response
            .json()
            .await
            .with_context(|| format!("Failed to parse {} response", self.label))?;

        chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .with_context(|| format!("No response from {}", self.label))
    }

    async fn chat_stream(
        &self,
        messages: &[Message],
        tx: UnboundedSender<String>,
    ) -> Result<String> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: messages.to_vec(),
            temperature: Some(0.7),
            max_tokens: Some(MAX_TOKENS),
            stream: true,
        };
        let url = self.chat_url();

        let response = send_with_retry(
            || {
                self.with_auth(
                    self.client
                        .post(url.as_str())
                        .header("Content-Type", "application/json")
                        .json(&request),
                )
            },
            &self.label,
        )
        .await?;

        let mut full_response = String::new();
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.context("Failed to read stream chunk")?;
            let chunk_str = String::from_utf8_lossy(&chunk);

            // SSE format: "data: {...}\n\n" ("data:" without a space is tolerated).
            for line in chunk_str.lines() {
                let Some(data) = line
                    .strip_prefix("data: ")
                    .or_else(|| line.strip_prefix("data:"))
                else {
                    continue;
                };
                if data == "[DONE]" {
                    break;
                }
                if let Ok(parsed) = serde_json::from_str::<StreamChunk>(data) {
                    if let Some(choice) = parsed.choices.first() {
                        if let Some(content) = &choice.delta.content {
                            full_response.push_str(content);
                            // Forward token; if the receiver is gone, stop early.
                            if tx.send(content.clone()).is_err() {
                                return Ok(full_response);
                            }
                        }
                    }
                }
            }
        }

        Ok(full_response)
    }
}

// ---------------------------------------------------------------------------
// Anthropic client
// ---------------------------------------------------------------------------

const ANTHROPIC_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    delta: Option<AnthropicStreamDelta>,
}

#[derive(Debug, Deserialize)]
struct AnthropicStreamDelta {
    text: Option<String>,
}

/// Client for Anthropic's Messages API.
///
/// Differs from the OpenAI-compatible shape: the leading `system` message is
/// hoisted to a top-level field, `max_tokens` is required, auth is via
/// `x-api-key`, and streaming uses Anthropic's own SSE events.
#[derive(Debug, Clone)]
pub struct AnthropicClient {
    client: reqwest::Client,
    api_key: String,
    model: String,
    ctx_window: usize,
}

impl AnthropicClient {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>, ctx_window: usize) -> Self {
        Self {
            client: http_client(),
            api_key: api_key.into(),
            model: model.into(),
            ctx_window,
        }
    }

    /// Split `messages` into the top-level `system` string (any `role=="system"`
    /// messages, concatenated) and the user/assistant turn list.
    fn split_messages(messages: &[Message]) -> (Option<String>, Vec<AnthropicMessage>) {
        let mut system: Option<String> = None;
        let mut turns = Vec::new();
        for m in messages {
            if m.role == "system" {
                system = Some(match system {
                    Some(existing) => format!("{existing}\n\n{}", m.content),
                    None => m.content.clone(),
                });
                continue;
            }
            let role = if m.role == "assistant" {
                "assistant"
            } else {
                "user"
            };
            turns.push(AnthropicMessage {
                role: role.to_string(),
                content: m.content.clone(),
            });
        }
        (system, turns)
    }

    fn build_request(&self, messages: &[Message], stream: bool) -> AnthropicRequest {
        let (system, turns) = Self::split_messages(messages);
        AnthropicRequest {
            model: self.model.clone(),
            max_tokens: MAX_TOKENS,
            system,
            messages: turns,
            stream,
        }
    }

    fn send(&self, request: &AnthropicRequest) -> reqwest::RequestBuilder {
        self.client
            .post(ANTHROPIC_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("Content-Type", "application/json")
            .json(request)
    }

    async fn chat(&self, messages: &[Message]) -> Result<String> {
        let request = self.build_request(messages, false);
        let response = send_with_retry(|| self.send(&request), "Anthropic").await?;

        let parsed: AnthropicResponse = response
            .json()
            .await
            .context("Failed to parse Anthropic response")?;

        parsed
            .content
            .into_iter()
            .find_map(|c| c.text)
            .context("No response from Anthropic")
    }

    async fn chat_stream(
        &self,
        messages: &[Message],
        tx: UnboundedSender<String>,
    ) -> Result<String> {
        let request = self.build_request(messages, true);
        let response = send_with_retry(|| self.send(&request), "Anthropic").await?;

        let mut full_response = String::new();
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.context("Failed to read stream chunk")?;
            let chunk_str = String::from_utf8_lossy(&chunk);

            // Anthropic SSE: interleaved "event:" and "data:" lines. We only need
            // the JSON on the "data:" lines; accumulate `content_block_delta`
            // events' `delta.text`.
            for line in chunk_str.lines() {
                let Some(data) = line
                    .strip_prefix("data: ")
                    .or_else(|| line.strip_prefix("data:"))
                else {
                    continue;
                };
                if let Ok(event) = serde_json::from_str::<AnthropicStreamEvent>(data) {
                    if event.event_type == "content_block_delta" {
                        if let Some(text) = event.delta.and_then(|d| d.text) {
                            full_response.push_str(&text);
                            if tx.send(text).is_err() {
                                return Ok(full_response);
                            }
                        }
                    }
                }
            }
        }

        Ok(full_response)
    }
}

// ---------------------------------------------------------------------------
// Provider (enum dispatch)
// ---------------------------------------------------------------------------

/// A resolved, ready-to-call LLM provider. Construct via
/// [`crate::config::Config::resolve_provider`].
#[derive(Debug, Clone)]
pub enum Provider {
    OpenAiCompat(OpenAiCompatClient),
    Anthropic(AnthropicClient),
}

impl Provider {
    /// Human-readable provider label (e.g. `"Groq"`, `"OpenAI"`, `"Anthropic"`).
    /// Part of the public provider API; consumed by the follow-up Config pane.
    #[allow(dead_code)]
    pub fn label(&self) -> &str {
        match self {
            Provider::OpenAiCompat(c) => &c.label,
            Provider::Anthropic(_) => "Anthropic",
        }
    }

    /// The active model id.
    /// Part of the public provider API; consumed by the follow-up Config pane.
    #[allow(dead_code)]
    pub fn model(&self) -> &str {
        match self {
            Provider::OpenAiCompat(c) => &c.model,
            Provider::Anthropic(c) => &c.model,
        }
    }

    /// Context window (tokens) for the active model.
    pub fn context_window(&self) -> usize {
        match self {
            Provider::OpenAiCompat(c) => c.ctx_window,
            Provider::Anthropic(c) => c.ctx_window,
        }
    }

    /// Available context characters for RAG, given current usage. Mirrors the
    /// former `GroqClient::available_context_chars` (~4 chars/token estimate).
    pub fn available_context_chars(
        &self,
        system_chars: usize,
        conversation_chars: usize,
        reserved_response_tokens: usize,
    ) -> usize {
        let total_tokens = self.context_window();
        let used_tokens = (system_chars + conversation_chars) / 4;
        let available_tokens = total_tokens.saturating_sub(used_tokens + reserved_response_tokens);
        available_tokens * 4
    }

    /// Send a chat (non-streaming) and return the full response.
    pub async fn chat(&self, messages: &[Message]) -> Result<String> {
        match self {
            Provider::OpenAiCompat(c) => c.chat(messages).await,
            Provider::Anthropic(c) => c.chat(messages).await,
        }
    }

    /// Stream a chat, forwarding each token over `tx`; returns the full response.
    pub async fn chat_stream(
        &self,
        messages: &[Message],
        tx: UnboundedSender<String>,
    ) -> Result<String> {
        match self {
            Provider::OpenAiCompat(c) => c.chat_stream(messages, tx).await,
            Provider::Anthropic(c) => c.chat_stream(messages, tx).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    // ---- pure logic (no I/O) --------------------------------------------

    #[test]
    fn provider_kind_parses_and_rejects() {
        assert_eq!("groq".parse::<ProviderKind>().unwrap(), ProviderKind::Groq);
        assert_eq!(
            " OpenAI ".parse::<ProviderKind>().unwrap(),
            ProviderKind::OpenAi
        );
        assert!("gemini".parse::<ProviderKind>().is_err());
    }

    #[test]
    fn default_and_ctx_lookups() {
        assert_eq!(default_model(ProviderKind::Groq), "openai/gpt-oss-120b");
        assert_eq!(default_model(ProviderKind::Anthropic), "claude-sonnet-5");
        // Known model → registry window; unknown → per-provider fallback.
        assert_eq!(ctx_for(ProviderKind::OpenAi, "gpt-4o"), 128000);
        assert_eq!(ctx_for(ProviderKind::Groq, "some-custom-model"), 8192);
    }

    #[test]
    fn anthropic_hoists_system_and_maps_roles() {
        let msgs = vec![
            Message {
                role: "system".into(),
                content: "sys-a".into(),
            },
            Message {
                role: "system".into(),
                content: "sys-b".into(),
            },
            Message {
                role: "user".into(),
                content: "hi".into(),
            },
            Message {
                role: "assistant".into(),
                content: "hello".into(),
            },
        ];
        let (system, turns) = AnthropicClient::split_messages(&msgs);
        assert_eq!(system.as_deref(), Some("sys-a\n\nsys-b"));
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0].role, "user");
        assert_eq!(turns[1].role, "assistant");
    }

    // ---- mock-HTTP round trip (B-002) -----------------------------------

    /// Serve one canned HTTP response per queued script entry, each on its own
    /// `Connection: close` socket, and return the base URL to point a client at.
    async fn spawn_mock(scripts: Vec<(u16, &'static str, String)>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            for (code, reason, body) in scripts {
                let (mut sock, _) = listener.accept().await.unwrap();
                drain_request(&mut sock).await;
                let resp = format!(
                    "HTTP/1.1 {code} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = sock.write_all(resp.as_bytes()).await;
                let _ = sock.shutdown().await;
            }
        });
        format!("http://127.0.0.1:{port}")
    }

    /// Read an HTTP request far enough to consume its body (so the client sees a
    /// clean response rather than a connection reset).
    async fn drain_request(sock: &mut tokio::net::TcpStream) {
        let mut buf = Vec::new();
        let mut tmp = [0u8; 1024];
        loop {
            let n = sock.read(&mut tmp).await.unwrap_or(0);
            if n == 0 {
                return;
            }
            buf.extend_from_slice(&tmp[..n]);
            if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&buf[..pos]).to_lowercase();
                let content_len = headers
                    .lines()
                    .find_map(|l| l.strip_prefix("content-length:"))
                    .and_then(|v| v.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                if buf.len() - (pos + 4) >= content_len {
                    return;
                }
            }
        }
    }

    fn openai_provider(base_url: String) -> Provider {
        Provider::OpenAiCompat(OpenAiCompatClient::new(
            "Mock",
            base_url,
            Some("test-key".to_string()),
            "mock-model",
            8192,
        ))
    }

    #[tokio::test]
    async fn openai_compat_chat_parses_response() {
        let body = r#"{"choices":[{"message":{"role":"assistant","content":"hello from mock"}}]}"#;
        let url = spawn_mock(vec![(200, "OK", body.to_string())]).await;
        let provider = openai_provider(url);

        let out = provider
            .chat(&[Message {
                role: "user".into(),
                content: "hi".into(),
            }])
            .await
            .unwrap();
        assert_eq!(out, "hello from mock");
    }

    #[tokio::test]
    async fn openai_compat_retries_5xx_then_succeeds() {
        let body = r#"{"choices":[{"message":{"role":"assistant","content":"recovered"}}]}"#;
        // First attempt 500 (retryable), second attempt 200 (B-005 retry path).
        let url = spawn_mock(vec![
            (500, "Internal Server Error", "boom".to_string()),
            (200, "OK", body.to_string()),
        ])
        .await;
        let provider = openai_provider(url);

        let out = provider
            .chat(&[Message {
                role: "user".into(),
                content: "hi".into(),
            }])
            .await
            .unwrap();
        assert_eq!(out, "recovered");
    }

    #[tokio::test]
    async fn openai_compat_surfaces_client_error() {
        // 400 is non-retryable → error bubbles up with the status + body snippet.
        let url = spawn_mock(vec![(400, "Bad Request", "bad model".to_string())]).await;
        let provider = openai_provider(url);

        let err = provider
            .chat(&[Message {
                role: "user".into(),
                content: "hi".into(),
            }])
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("400"), "unexpected error: {err}");
        assert!(err.contains("bad model"), "unexpected error: {err}");
    }
}
