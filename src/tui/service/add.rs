//! Add (ingest) pane service (blueprint §4 — highest-effort pane).
//!
//! Real implementation reimplements the `commands/add.rs:110-425` loop:
//! `ingest::{requires_transcription,extract_from_file_async,fetch_url}` →
//! `chunk_text`/`ChunkConfig` → `embeddings::embed_text` →
//! `DocumentStore::{exists_by_path,insert}` → `ChunkStore::{init_schema,insert}`,
//! emitting [`Message::IngestProgress`] / [`Message::IngestFileDone`] as it goes
//! and a terminal [`Message::IngestComplete`]. **All DB + embedding work must run
//! inside `tokio::task::spawn_blocking`**; transcription/URL fetch are async.
//! Phase 2a is a stub.

use anyhow::Result;
use tokio::sync::mpsc::UnboundedSender;

use crate::tui::action::{Message, ToastLevel};

/// Ingest a local path or URL into the current bucket, streaming incremental
/// progress messages over `msg_tx`.
///
/// TODO(add-pane): drive the extract → chunk → embed → insert pipeline,
/// sending `Message::IngestProgress`/`IngestFileDone` per unit of work and a
/// final `Message::IngestComplete { added, skipped }`.
pub async fn start_ingest(
    source: String,
    is_url: bool,
    msg_tx: UnboundedSender<Message>,
) -> Result<()> {
    let _ = (source, is_url);
    let _ = msg_tx.send(Message::Toast {
        level: ToastLevel::Warn,
        text: "Add / ingest not implemented yet".to_string(),
    });
    Ok(())
}
