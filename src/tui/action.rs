//! Action/Message plumbing for the TUI (blueprint §1, §3).
//!
//! Two flat enums cross the async boundary:
//! - [`Action`] — user intent flowing from the UI/render loop into the worker.
//! - [`Message`] — worker results flowing back into the render loop's reducer.
//!
//! Both are `Clone` and carry only owned data so they can be sent across
//! `tokio` tasks freely. **This surface is frozen for Phase 2:** every pane's
//! full set of intents and results is declared here up front so pane agents
//! never have to touch this file. Later phases may add variants but should not
//! reshape existing ones. Some variants are unused until their pane lands
//! (hence the enum-level `#[allow(dead_code)]`).

use crate::llm::groq::Message as LlmMessage;

// ---------------------------------------------------------------------------
// Shared result payloads (owned; safe to send across tasks).
// ---------------------------------------------------------------------------

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

/// One hit from a full-text document search (Search pane).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SearchHit {
    pub id: i64,
    pub filename: String,
    pub content_type: String,
    /// A short excerpt around the match, ready to render.
    pub snippet: String,
}

/// A document row for the Docs list (Docs pane).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DocMeta {
    pub id: i64,
    pub filename: String,
    pub content_type: String,
    /// Preformatted timestamp (e.g. `%m/%d %H:%M`).
    pub created_at: String,
    pub tags: Option<String>,
    /// Character length of the stored content (cheap size hint).
    pub content_len: usize,
}

/// Full document content for the Docs detail view (Docs pane).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DocDetail {
    pub id: i64,
    pub filename: String,
    pub content_type: String,
    pub content: String,
}

/// A study item surfaced for quizzing or spaced-repetition review
/// (Quiz + Review panes).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StudyCard {
    pub id: i64,
    pub item_type: String,
    pub front: String,
    pub back: String,
}

/// Current configuration snapshot for the Config pane.
///
/// Provider-aware (E2-core): the pane derives its model list from
/// `provider::suggested_models(kind)` itself, so only the *active* selections
/// and the per-provider key-set flags travel here.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ConfigData {
    /// Currently selected provider id (`"groq"`/`"openai"`/`"anthropic"`/`"ollama"`).
    pub provider: String,
    /// The resolved default model id (field value or the provider default).
    pub model: String,
    /// The Ollama server root (config value or the built-in default).
    pub ollama_url: String,
    /// Whether a usable Groq credential exists (config field or env var).
    pub groq_key_set: bool,
    /// Whether a usable OpenAI credential exists (config field or env var).
    pub openai_key_set: bool,
    /// Whether a usable Anthropic credential exists (config field or env var).
    pub anthropic_key_set: bool,
}

// ---------------------------------------------------------------------------
// Action — user intent → worker.
// ---------------------------------------------------------------------------

/// User intent → worker. Each dispatched `Action` produces exactly one
/// terminal [`Message::ActionDone`] (plus any number of result messages).
///
/// Ownership of each variant by pane:
/// - global/library: [`Action::LoadLibrary`], [`Action::SwitchBucket`]
/// - Chat: [`Action::LoadConversations`], [`Action::SendChat`]
/// - Search: [`Action::RunSearch`]
/// - Docs: [`Action::LoadDocs`], [`Action::LoadDoc`], [`Action::DeleteDoc`]
/// - Add: [`Action::StartIngest`]
/// - Study: [`Action::GenerateStudy`]
/// - Quiz: [`Action::StartQuiz`], [`Action::GradeQuiz`]
/// - Review: [`Action::LoadDue`], [`Action::GradeReview`]
/// - Config: [`Action::LoadConfig`], [`Action::SaveConfig`]
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Action {
    // ---- global / library ----
    /// Load library stats (buckets, current bucket, doc/chunk counts, key, model).
    LoadLibrary,
    /// Switch the active bucket, then reload the library.
    SwitchBucket(String),

    // ---- chat ----
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

    // ---- search ----
    /// Run a full-text search over the current bucket's documents.
    /// → [`Message::SearchResults`]. Calls `DocumentStore::search`.
    RunSearch { query: String },

    // ---- docs ----
    /// Load the document list for the current bucket.
    /// → [`Message::DocsLoaded`]. Calls `DocumentStore::list`.
    LoadDocs,
    /// Load one document's full content for the detail view.
    /// → [`Message::DocLoaded`]. Calls `DocumentStore::get`.
    LoadDoc { id: i64 },
    /// Delete a document by id (after in-pane confirm).
    /// → [`Message::DocDeleted`]. Calls `DocumentStore::delete`.
    DeleteDoc { id: i64 },

    // ---- add ----
    /// Ingest a local file/directory path or a URL into the current bucket,
    /// emitting incremental [`Message::IngestProgress`] /
    /// [`Message::IngestFileDone`] and a terminal [`Message::IngestComplete`].
    /// Calls `ingest::*`, `chunk_text`, `embeddings::embed_text`,
    /// `DocumentStore::insert`, `ChunkStore::insert` — mirrors `commands/add.rs`.
    StartIngest { source: String, is_url: bool },

    // ---- study ----
    /// Generate study material (`kind` = notes/summary/flashcards/…) for a
    /// topic, streaming tokens as [`Message::StudyToken`] then
    /// [`Message::StudyDone`]. When `save_items` is set, parsed Q/A pairs are
    /// persisted via `StudyStore::bulk_insert`. Calls `commands/generate.rs`
    /// helpers (`prompts::*`, `get_document_context_pub`, `parse_qa_pairs`).
    GenerateStudy {
        kind: String,
        topic: String,
        save_items: bool,
    },

    // ---- quiz ----
    /// Generate a quiz of `count` questions from the current bucket's materials.
    /// → [`Message::QuizGenerated`]. Calls `get_document_context_pub`,
    /// `GroqClient::chat`, `parse_quiz_questions`.
    StartQuiz { count: usize },
    /// Grade a quiz answer with an SM-2 quality (0–5).
    /// → [`Message::QuizGraded`]. Calls `StudyStore::update_after_review`.
    GradeQuiz { id: i64, quality: u8 },

    // ---- review ----
    /// Load study items due for spaced-repetition review.
    /// → [`Message::DueLoaded`]. Calls `StudyStore::{count_due,get_due}`.
    LoadDue,
    /// Grade a review with an SM-2 quality (0–5).
    /// → [`Message::ReviewGraded`]. Calls `StudyStore::update_after_review`.
    GradeReview { id: i64, quality: u8 },

    // ---- config ----
    /// Load the current configuration snapshot.
    /// → [`Message::ConfigLoaded`]. Calls `Config::load` +
    /// `provider::suggested_models`.
    LoadConfig,
    /// Persist configuration changes for the given provider.
    ///
    /// `provider` names the active provider (also selects which `*_api_key`
    /// field a `Some(api_key)` is written into; Ollama has no key field). `model`
    /// overwrites `default_model` when `Some`; `ollama_url` overwrites the Ollama
    /// server root when `Some`. → [`Message::ConfigSaved`]. Calls `Config::save`.
    SaveConfig {
        provider: String,
        api_key: Option<String>,
        model: Option<String>,
        ollama_url: Option<String>,
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

// ---------------------------------------------------------------------------
// Message — worker result → render loop reducer.
// ---------------------------------------------------------------------------

/// Worker result → render loop reducer.
///
/// Ownership of each variant by pane:
/// - global/library: [`Message::LibraryLoaded`]
/// - Chat: [`Message::ConversationsLoaded`], [`Message::ChatToken`],
///   [`Message::ChatDone`], [`Message::ChatError`]
/// - Search: [`Message::SearchResults`]
/// - Docs: [`Message::DocsLoaded`], [`Message::DocLoaded`], [`Message::DocDeleted`]
/// - Add: [`Message::IngestProgress`], [`Message::IngestFileDone`],
///   [`Message::IngestComplete`]
/// - Study: [`Message::StudyToken`], [`Message::StudyDone`]
/// - Quiz: [`Message::QuizGenerated`], [`Message::QuizGraded`]
/// - Review: [`Message::DueLoaded`], [`Message::ReviewGraded`]
/// - Config: [`Message::ConfigLoaded`], [`Message::ConfigSaved`]
/// - cross-cutting: [`Message::Toast`], [`Message::ActionDone`]
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Message {
    // ---- global / library ----
    /// Library stats finished loading.
    LibraryLoaded {
        buckets: Vec<String>,
        current: Option<String>,
        doc_count: i64,
        chunk_count: i64,
        has_api_key: bool,
        model: String,
    },

    // ---- chat ----
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

    // ---- search ----
    /// Search finished; carries the query it was for (to guard against races)
    /// and the ranked hits.
    SearchResults { query: String, hits: Vec<SearchHit> },

    // ---- docs ----
    /// The document list finished loading.
    DocsLoaded(Vec<DocMeta>),
    /// One document's full content finished loading (detail view).
    DocLoaded(DocDetail),
    /// A document was deleted.
    DocDeleted { id: i64 },

    // ---- add ----
    /// Incremental ingest progress (files or chunks completed so far).
    IngestProgress {
        done: usize,
        total: usize,
        current: String,
    },
    /// One source file finished ingesting.
    IngestFileDone { filename: String, chunks: usize },
    /// The whole ingest run finished.
    IngestComplete { added: usize, skipped: usize },

    // ---- study ----
    /// One streamed study-generation token.
    StudyToken(String),
    /// Study generation finished; `saved_items` = flashcards persisted.
    StudyDone { content: String, saved_items: usize },

    // ---- quiz ----
    /// A generated quiz is ready.
    QuizGenerated(Vec<StudyCard>),
    /// A quiz answer was graded (SM-2 applied).
    QuizGraded { id: i64 },

    // ---- review ----
    /// Study items due for review finished loading.
    DueLoaded(Vec<StudyCard>),
    /// A review was graded (SM-2 applied).
    ReviewGraded { id: i64 },

    // ---- config ----
    /// Current configuration finished loading.
    ConfigLoaded(ConfigData),
    /// Configuration changes were saved.
    ConfigSaved,

    // ---- cross-cutting ----
    /// A transient status toast.
    Toast { level: ToastLevel, text: String },
    /// Sentinel emitted at the end of every dispatched action so the loop can
    /// decrement its in-flight counter regardless of success or failure.
    ActionDone,
}
