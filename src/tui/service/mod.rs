//! TUI-facing service layer (blueprint §1, §4).
//!
//! Every function here is safe to call from the async render/worker world:
//! all `rusqlite` + `fastembed` work is confined to `tokio::task::spawn_blocking`
//! closures that each open their own [`Database`] (never held across `.await`,
//! never shared with the render loop). Streaming chat is orchestrated here via
//! [`crate::llm::provider::Provider::chat_stream`].
//!
//! These call the *lower-level* stores/LLM/embeddings directly — never the
//! interactive `commands::*::run()` orchestrators (which print + prompt).
//!
//! # Per-pane services (Phase 2)
//!
//! Chat + library live in this file (unchanged). Every other pane owns a
//! sibling module — [`search`], [`docs`], [`add`], [`study`], [`quiz`],
//! [`review`], [`config`] — whose function *signatures* are what
//! [`super::worker::dispatch`] calls. Phase 2a ships those as minimal safe
//! stubs; a pane agent fills in the body of its own module only. **All DB /
//! embedding work in these modules must run inside `tokio::task::spawn_blocking`
//! closures that open their own [`Database`]** — never held across `.await`.

pub mod add;
pub mod config;
pub mod docs;
pub mod quiz;
pub mod review;
pub mod search;
pub mod study;

use anyhow::Result;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task;

use crate::bucket::{self, Bucket};
use crate::commands::chat::{build_fts_context, build_grounded_context};
use crate::config::Config;
use crate::llm::Message as LlmMessage;
use crate::retrieval::Citation;
use crate::storage::{ChunkStore, ConversationStore, Database, DocumentStore};

use super::action::{ConversationMeta, Message};

const GROUNDED_SYSTEM_PROMPT: &str = r#"You are The Librarian, a knowledgeable study assistant helping a student learn from their course materials.

IMPORTANT INSTRUCTIONS:
1. Answer questions primarily using the provided context from their documents
2. When the context contains relevant information, use it as the foundation for your answer and cite the source
3. If asked about exercises, problems, or questions from the materials, use the textbook knowledge in the context to reason through the answer
4. You may use your general knowledge to supplement and explain concepts from the materials, but always prioritize what's in the provided context
5. If the context has no relevant information at all, say so but still try to help using general knowledge, noting that you're going beyond their materials

RESPONSE STYLE:
- Answer ONLY what was asked. Do not add unrequested extras like LaTeX snippets, assignment templates, submission advice, or formatting suggestions
- Keep answers focused and direct. If someone asks about a problem, explain the solution — don't write their homework for them
- Do not assume the student wants code, LaTeX, or any specific output format unless they explicitly ask for it
- Use plain text with clear formatting. Only use code blocks if the question involves actual code

CITATIONS:
- The context is split into numbered sources, each headed by a marker like [Source 2: notes.pdf (chunk 5)]
- When a statement draws on a source, cite it inline with just its number in brackets, e.g. [Source 2]
- Cite only source numbers that actually appear in the provided context; never invent one"#;

const NO_DOCS_SYSTEM_PROMPT: &str = r#"You are The Librarian, a knowledgeable study assistant. The user has no documents loaded in their current library.

Help them by:
1. Answering general questions to the best of your ability
2. Suggesting they add study materials with 'librarian add <file>'
3. Being clear when you're using general knowledge vs. their specific materials"#;

/// Pick the system prompt for a fresh conversation based on whether the current
/// bucket has any documents.
pub fn system_prompt(has_docs: bool) -> &'static str {
    if has_docs {
        GROUNDED_SYSTEM_PROMPT
    } else {
        NO_DOCS_SYSTEM_PROMPT
    }
}

/// Load library stats for the Home screen and global sidebar.
pub async fn load_library() -> Result<Message> {
    task::spawn_blocking(|| {
        let buckets = Bucket::list_all().unwrap_or_default();
        let current = bucket::get_current_bucket().ok().flatten().map(|b| b.name);

        let config = Config::load().unwrap_or_default();
        let has_api_key = config.has_api_key();
        let model = config.resolved_model();

        let (doc_count, chunk_count) = match Database::open() {
            Ok(db) => {
                let doc_count = DocumentStore::new(&db).count().unwrap_or(0);
                let chunk_store = ChunkStore::new(&db);
                chunk_store.init_schema().ok();
                let chunk_count = chunk_store.count().unwrap_or(0);
                (doc_count, chunk_count)
            }
            Err(_) => (0, 0),
        };

        Ok(Message::LibraryLoaded {
            buckets,
            current,
            doc_count,
            chunk_count,
            has_api_key,
            model,
        })
    })
    .await?
}

/// Switch the active bucket, then reload the library stats.
pub async fn switch_bucket(name: String) -> Result<Message> {
    task::spawn_blocking(move || bucket::set_current_bucket(Some(&name))).await??;
    load_library().await
}

/// Load recent conversations for the current bucket.
pub async fn load_conversations() -> Result<Vec<ConversationMeta>> {
    task::spawn_blocking(|| {
        let db = Database::open()?;
        let store = ConversationStore::new(&db);
        let recent = store.list_recent(20)?;
        let metas = recent
            .into_iter()
            .map(|c| ConversationMeta {
                id: c.id,
                title: c.title.unwrap_or_else(|| "(untitled)".to_string()),
                updated_at: c.updated_at.format("%m/%d %H:%M").to_string(),
            })
            .collect();
        Ok(metas)
    })
    .await?
}

/// Orchestrate one streaming chat turn end to end:
/// 1. build RAG context off-thread,
/// 2. inject it into the last user message,
/// 3. stream tokens back over `msg_tx` as [`Message::ChatToken`],
/// 4. persist the plain turn off-thread,
/// 5. emit [`Message::ChatDone`].
pub async fn run_chat(
    conversation_id: Option<i64>,
    history: Vec<LlmMessage>,
    question: String,
    is_first: bool,
    msg_tx: UnboundedSender<Message>,
) -> Result<()> {
    let config = Config::load()?;
    let provider = config.resolve_provider()?;

    // 1. Build retrieval context on a blocking thread (opens its own DB).
    let q_for_ctx = question.clone();
    let (context, citations) = task::spawn_blocking(move || build_context(&q_for_ctx)).await??;

    // 2. Assemble the API message list; inject context into the final user turn.
    let mut api_messages = history;
    if !context.is_empty()
        && let Some(last) = api_messages.last_mut()
    {
        last.content = format!(
            "CONTEXT FROM YOUR STUDY MATERIALS:\n{}\n\n---\n\nQUESTION: {}",
            context, question
        );
    }

    // 3. Stream tokens. Run the SSE loop concurrently with the forwarding loop
    //    so tokens reach the render loop live.
    let (tok_tx, mut tok_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let stream_provider = provider.clone();
    let stream_handle =
        tokio::spawn(async move { stream_provider.chat_stream(&api_messages, tok_tx).await });

    while let Some(token) = tok_rx.recv().await {
        let _ = msg_tx.send(Message::ChatToken(token));
    }
    let response = stream_handle.await??;

    // 4. Persist the plain question + response off-thread.
    let response_for_persist = response.clone();
    let conv_id = task::spawn_blocking(move || {
        persist_turn(conversation_id, is_first, &question, &response_for_persist)
    })
    .await??;

    // 5. Signal completion.
    let _ = msg_tx.send(Message::ChatDone {
        conversation_id: conv_id,
        response,
        citations,
    });
    Ok(())
}

/// Build hybrid RAG context for a question from the current bucket, plus the
/// structured citations grounding it (B-011). Runs on a blocking thread; opens
/// its own [`Database`].
fn build_context(question: &str) -> Result<(String, Vec<Citation>)> {
    let db = Database::open()?;
    let doc_store = DocumentStore::new(&db);
    let chunk_store = ChunkStore::new(&db);
    chunk_store.init_schema()?;

    let doc_count = doc_store.count().unwrap_or(0);
    if doc_count == 0 {
        return Ok((String::new(), Vec::new()));
    }
    let chunk_count = chunk_store.count().unwrap_or(0);

    // A fixed, generous budget — the TUI does not thread through live token
    // accounting in Phase 1.
    let max_context = 8000usize;
    let enhanced = crate::search::enhance_query(question);

    if chunk_count > 0 {
        build_grounded_context(&chunk_store, &doc_store, &enhanced, max_context)
    } else {
        let context = build_fts_context(&doc_store, question, max_context)?;
        Ok((context, Vec::new()))
    }
}

/// Persist a completed turn: create the conversation if needed, set its title on
/// the first turn, and append the user + assistant messages. Runs on a blocking
/// thread; opens its own [`Database`]. Returns the conversation id.
fn persist_turn(
    conversation_id: Option<i64>,
    is_first: bool,
    question: &str,
    response: &str,
) -> Result<i64> {
    let db = Database::open()?;
    let store = ConversationStore::new(&db);

    let id = match conversation_id {
        Some(id) => id,
        None => store.create(None)?,
    };

    if is_first {
        let title: String = question.chars().take(60).collect();
        let title = match title.rfind(' ') {
            Some(pos) if pos > 0 => title[..pos].to_string(),
            _ => title,
        };
        store.update_title(id, &title)?;
    }

    store.add_message(id, "user", question)?;
    store.add_message(id, "assistant", response)?;
    Ok(id)
}
