//! Study pane service (blueprint §4).
//!
//! Streams generated study material via
//! [`crate::llm::provider::Provider::chat_stream`] (same pattern as
//! [`super::run_chat`]): build the document context off-thread
//! with [`get_document_context_pub`], pick the system prompt from
//! [`crate::commands::generate::prompts`] by `kind`, assemble a system + user
//! message pair, stream each token back as [`Message::StudyToken`], then emit a
//! terminal [`Message::StudyDone`]. When `save_items` is set and the kind
//! produces Q/A pairs (flashcards/quiz), parsed pairs are persisted via
//! [`StudyStore::bulk_insert`].
//!
//! Every `rusqlite` / DB touch runs inside `tokio::task::spawn_blocking` on its
//! own [`Database`] — never held across `.await`.
//!
//! Note: the TUI flow deliberately does *not* write the generated file to disk
//! (the interactive `commands::generate` path does that). Surfacing the content
//! in-pane is enough; `get_save_path` + `fs::write` are left to the CLI flow.

use anyhow::Result;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task;

use crate::commands::generate::{get_document_context_pub, parse_qa_pairs, prompts};
use crate::config::Config;
use crate::llm::Message as LlmMessage;
use crate::storage::{Database, StudyStore};
use crate::tui::action::{Message, ToastLevel};

/// Resolve `(system_prompt, display_name)` for a study `kind`. Unknown kinds
/// fall back to the study guide.
fn resolve_kind(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "flashcards" => (prompts::FLASHCARDS, "Flashcards"),
        "quiz" => (prompts::QUIZ, "Quiz"),
        "summary" => (prompts::SUMMARY, "Summary"),
        "homework" => (prompts::HOMEWORK_HELP, "Homework Help"),
        _ => (prompts::STUDY_GUIDE, "Study Guide"),
    }
}

/// Generate study material for `topic`, streaming [`Message::StudyToken`]s and a
/// terminal [`Message::StudyDone`]. On flashcards/quiz with `save_items`, parsed
/// pairs are persisted and counted into `StudyDone.saved_items`.
pub async fn generate_study(
    kind: String,
    topic: String,
    save_items: bool,
    msg_tx: UnboundedSender<Message>,
) -> Result<()> {
    let config = Config::load()?;
    let provider = config.resolve_provider()?;

    let (system_prompt, name) = resolve_kind(&kind);

    // 1. Build the document context on a blocking thread (opens its own DB).
    let topic_for_ctx = topic.clone();
    let context = task::spawn_blocking(move || get_document_context_pub(&topic_for_ctx)).await??;
    if context.is_empty() {
        anyhow::bail!(
            "No documents in the current bucket. Add materials first with `librarian add`."
        );
    }

    // 2. Assemble the system + user messages (mirrors `commands::generate`).
    let user_message = if topic.is_empty() {
        format!(
            "Create a {} from the following course materials:\n\n{}",
            name.to_lowercase(),
            context
        )
    } else {
        format!(
            "Create a {} focused on '{}' from the following course materials:\n\n{}",
            name.to_lowercase(),
            topic,
            context
        )
    };
    let messages = vec![
        LlmMessage {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        },
        LlmMessage {
            role: "user".to_string(),
            content: user_message,
        },
    ];

    // 3. Stream tokens: run the SSE loop concurrently with the forwarding loop so
    //    tokens reach the render loop live.
    let (tok_tx, mut tok_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let stream_provider = provider.clone();
    let stream_handle =
        tokio::spawn(async move { stream_provider.chat_stream(&messages, tok_tx).await });

    while let Some(token) = tok_rx.recv().await {
        let _ = msg_tx.send(Message::StudyToken(token));
    }
    let content = stream_handle.await??;

    // 4. Optionally persist parsed Q/A pairs (flashcards/quiz only) off-thread.
    let mut saved_items = 0usize;
    if save_items && (kind == "flashcards" || kind == "quiz") {
        let content_for_parse = content.clone();
        let name_owned = name.to_string();
        saved_items = task::spawn_blocking(move || -> Result<usize> {
            let items = parse_qa_pairs(&name_owned, &content_for_parse);
            if items.is_empty() {
                return Ok(0);
            }
            let db = Database::open()?;
            let store = StudyStore::new(&db);
            let bulk: Vec<(Option<i64>, &str, &str, &str)> = items
                .iter()
                .map(|(item_type, front, back)| {
                    (None, item_type.as_str(), front.as_str(), back.as_str())
                })
                .collect();
            store.bulk_insert(&bulk)
        })
        .await??;
    }

    // 5. Surface a completion toast (also carries the saved-item count, since the
    //    pane state has no dedicated field for it).
    let toast = if saved_items > 0 {
        format!("{name} ready — saved {saved_items} items for review")
    } else {
        format!("{name} ready")
    };
    let _ = msg_tx.send(Message::Toast {
        level: ToastLevel::Success,
        text: toast,
    });

    // 6. Signal completion.
    let _ = msg_tx.send(Message::StudyDone {
        content,
        saved_items,
    });
    Ok(())
}
