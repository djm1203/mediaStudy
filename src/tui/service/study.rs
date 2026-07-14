//! Study pane service (blueprint §4).
//!
//! Real implementation streams generated study material (via a new
//! `generate_stream_tx` mirroring `chat_stream_tx`) using `commands/generate.rs`
//! helpers (`prompts::*`, `get_document_context_pub`), then — when requested —
//! parses Q/A pairs (`parse_qa_pairs`) and persists them with
//! `StudyStore::bulk_insert`. Phase 2a is a stub.

use anyhow::Result;
use tokio::sync::mpsc::UnboundedSender;

use crate::tui::action::{Message, ToastLevel};

/// Generate study material for `topic`, streaming [`Message::StudyToken`]s and a
/// terminal [`Message::StudyDone`].
///
/// TODO(study-pane): build the prompt via `prompts::*` + document context, call
/// `generate_stream_tx`, forward tokens, optionally `bulk_insert` parsed items.
pub async fn generate_study(
    kind: String,
    topic: String,
    save_items: bool,
    msg_tx: UnboundedSender<Message>,
) -> Result<()> {
    let _ = (kind, topic, save_items);
    let _ = msg_tx.send(Message::Toast {
        level: ToastLevel::Warn,
        text: "Study generation not implemented yet".to_string(),
    });
    Ok(())
}
