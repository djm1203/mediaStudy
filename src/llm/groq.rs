use anyhow::{Context, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::io::Write;

const GROQ_API_URL: &str = "https://api.groq.com/openai/v1/chat/completions";

#[derive(Debug, Clone)]
pub struct GroqClient {
    client: reqwest::Client,
    api_key: String,
    pub model: String,
}

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

/// Streaming response chunk
#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: Delta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    content: Option<String>,
}

impl GroqClient {
    /// Available models on Groq
    pub const MODELS: &'static [(&'static str, &'static str)] = &[
        ("openai/gpt-oss-120b", "GPT-OSS 120B - Most powerful"),
        ("llama-3.3-70b-versatile", "Llama 3.3 70B - Best for complex tasks"),
        ("llama-3.1-8b-instant", "Llama 3.1 8B - Fast and efficient"),
        ("mixtral-8x7b-32768", "Mixtral 8x7B - Good balance"),
        ("gemma2-9b-it", "Gemma 2 9B - Google's model"),
    ];

    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model: model.unwrap_or_else(|| "openai/gpt-oss-120b".to_string()),
        }
    }

    /// Send a chat message and get a response (non-streaming)
    pub async fn chat(&self, messages: &[Message]) -> Result<String> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: messages.to_vec(),
            temperature: Some(0.7),
            max_tokens: Some(4096),
            stream: false,
        };

        let response = self
            .client
            .post(GROQ_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Groq")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Groq API error ({}): {}", status, text);
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .context("Failed to parse Groq response")?;

        chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .context("No response from Groq")
    }

    /// Send a chat message with streaming response
    /// Prints tokens as they arrive and returns the complete response
    pub async fn chat_stream(&self, messages: &[Message]) -> Result<String> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: messages.to_vec(),
            temperature: Some(0.7),
            max_tokens: Some(4096),
            stream: true,
        };

        let response = self
            .client
            .post(GROQ_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Groq")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Groq API error ({}): {}", status, text);
        }

        let mut full_response = String::new();
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.context("Failed to read stream chunk")?;
            let chunk_str = String::from_utf8_lossy(&chunk);

            // SSE format: "data: {...}\n\n"
            for line in chunk_str.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        break;
                    }

                    if let Ok(parsed) = serde_json::from_str::<StreamChunk>(data) {
                        if let Some(choice) = parsed.choices.first() {
                            if let Some(content) = &choice.delta.content {
                                // Print token immediately
                                print!("{}", content);
                                std::io::stdout().flush().ok();
                                full_response.push_str(content);
                            }
                        }
                    }
                }
            }
        }

        // Print newline after streaming completes
        println!();

        Ok(full_response)
    }

    /// Simple single-turn query
    #[allow(dead_code)]
    pub async fn query(&self, prompt: &str) -> Result<String> {
        let messages = vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }];
        self.chat(&messages).await
    }

    /// Query with a system prompt
    #[allow(dead_code)]
    pub async fn query_with_system(&self, system: &str, user: &str) -> Result<String> {
        let messages = vec![
            Message {
                role: "system".to_string(),
                content: system.to_string(),
            },
            Message {
                role: "user".to_string(),
                content: user.to_string(),
            },
        ];
        self.chat(&messages).await
    }
}
