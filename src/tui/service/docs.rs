//! Docs pane service (blueprint §4).
//!
//! Real implementation calls `DocumentStore::{list,get,delete}` inside
//! `tokio::task::spawn_blocking`. Phase 2a is a stub.

use anyhow::Result;

use crate::tui::action::{Message, ToastLevel};

/// Load the document list for the current bucket as [`Message::DocsLoaded`].
///
/// TODO(docs-pane): `spawn_blocking` → `DocumentStore::list()` → map to
/// `DocMeta` → `Message::DocsLoaded(metas)`.
pub async fn load_docs() -> Result<Message> {
    Ok(Message::Toast {
        level: ToastLevel::Warn,
        text: "Docs not implemented yet".to_string(),
    })
}

/// Load one document's full content as [`Message::DocLoaded`].
///
/// TODO(docs-pane): `spawn_blocking` → `DocumentStore::get(id)` →
/// `Message::DocLoaded(detail)`.
pub async fn load_doc(id: i64) -> Result<Message> {
    let _ = id;
    Ok(Message::Toast {
        level: ToastLevel::Warn,
        text: "Docs not implemented yet".to_string(),
    })
}

/// Delete a document by id, returning [`Message::DocDeleted`].
///
/// TODO(docs-pane): `spawn_blocking` → `DocumentStore::delete(id)` →
/// `Message::DocDeleted { id }`.
pub async fn delete_doc(id: i64) -> Result<Message> {
    let _ = id;
    Ok(Message::Toast {
        level: ToastLevel::Warn,
        text: "Delete not implemented yet".to_string(),
    })
}
