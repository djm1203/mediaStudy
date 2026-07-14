//! Search pane service (blueprint §4).
//!
//! Real implementation calls `DocumentStore::search` (FTS over the current
//! bucket) inside `tokio::task::spawn_blocking`, mapping each [`Document`] to a
//! [`SearchHit`] with a short excerpt. Phase 2a is a stub.
//!
//! [`Document`]: crate::storage::documents::Document

use anyhow::Result;

use crate::tui::action::{Message, ToastLevel};

/// Run a full-text search and return ranked [`crate::tui::action::SearchHit`]s
/// as [`Message::SearchResults`].
///
/// TODO(search-pane): open `Database` in `spawn_blocking`, call
/// `DocumentStore::search(&query)`, build snippets, return
/// `Message::SearchResults { query, hits }`.
pub async fn run_search(query: String) -> Result<Message> {
    let _ = query;
    Ok(Message::Toast {
        level: ToastLevel::Warn,
        text: "Search not implemented yet".to_string(),
    })
}
