//! Action/Message plumbing for the TUI (blueprint §1, §3).
//!
//! Two flat enums cross the async boundary:
//! - [`Action`] — user intent flowing from the UI/render loop into the worker.
//! - [`Message`] — worker results flowing back into the render loop's reducer.
//!
//! Both are `Clone` and carry only owned data so they can be sent across
//! `tokio` tasks freely. Freeze these shapes early — later phases extend them
//! with new variants, they should not reshape existing ones.

use crate::llm::groq::Message as LlmMessage;

/// A conversation summary shown in the Chat screen's recent list.
#[derive(Debug, Clone)]
pub struct ConversationMeta {
    // Read once conversation-resume UI lands in Phase 2; kept in the frozen
    // data model now so the shape does not change later.
    #[allow(dead_code)]
    pub id: i64,
    pub title: String,
    pub updated_at: String,
}

/// User intent → worker. Each dispatched `Action` produces exactly one
/// terminal [`Message::ActionDone`] (plus any number of result messages).
#[derive(Debug, Clone)]
pub enum Action {
    /// Load library stats (buckets, current bucket, doc/chunk counts, key, model).
    LoadLibrary,
    /// Switch the active bucket, then reload the library.
    SwitchBucket(String),
    /// Load the recent conversation list for the Chat screen.
    LoadConversations,
    /// Send a chat turn: build RAG context, stream the reply, persist the turn.
    ///
    /// `history` is the full message list (system prompt at index 0, then prior
    /// turns, with the just-entered user question as the last element). The
    /// worker injects retrieved context into that final user message before
    /// calling the model, and persists the plain `question`/response afterwards.
    SendChat {
        conversation_id: Option<i64>,
        history: Vec<LlmMessage>,
        question: String,
        is_first: bool,
    },
}

/// Severity for transient status toasts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    Info,
    Success,
    Warn,
    Error,
}

/// Worker result → render loop reducer.
#[derive(Debug, Clone)]
pub enum Message {
    /// Library stats finished loading.
    LibraryLoaded {
        buckets: Vec<String>,
        current: Option<String>,
        doc_count: i64,
        chunk_count: i64,
        has_api_key: bool,
        model: String,
    },
    /// Recent conversations finished loading.
    ConversationsLoaded(Vec<ConversationMeta>),
    /// One streamed chat token (a `delta.content` fragment).
    ChatToken(String),
    /// A chat turn finished streaming and was persisted.
    ChatDone {
        conversation_id: i64,
        response: String,
    },
    /// A chat turn failed.
    ChatError(String),
    /// A transient status toast.
    Toast { level: ToastLevel, text: String },
    /// Sentinel emitted at the end of every dispatched action so the loop can
    /// decrement its in-flight counter regardless of success or failure.
    ActionDone,
}
