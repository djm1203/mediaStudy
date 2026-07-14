//! Docs pane service (blueprint §4).
//!
//! Calls `DocumentStore::{list,get,delete}` inside `tokio::task::spawn_blocking`
//! closures, each opening its own [`Database`] — never held across `.await`.

use anyhow::{Result, anyhow};
use tokio::task;

use crate::storage::{Database, DocumentStore};
use crate::tui::action::{DocDetail, DocMeta, Message};

/// Load the document list for the current bucket as [`Message::DocsLoaded`].
pub async fn load_docs() -> Result<Message> {
    task::spawn_blocking(|| {
        let db = Database::open()?;
        let store = DocumentStore::new(&db);
        let docs = store.list()?;
        let metas = docs
            .into_iter()
            .map(|d| DocMeta {
                id: d.id,
                filename: d.filename,
                content_type: d.content_type,
                created_at: d.created_at.format("%m/%d %H:%M").to_string(),
                tags: d.tags,
                content_len: d.content.chars().count(),
            })
            .collect();
        Ok(Message::DocsLoaded(metas))
    })
    .await?
}

/// Load one document's full content as [`Message::DocLoaded`].
pub async fn load_doc(id: i64) -> Result<Message> {
    task::spawn_blocking(move || {
        let db = Database::open()?;
        let store = DocumentStore::new(&db);
        let doc = store
            .get(id)?
            .ok_or_else(|| anyhow!("Document {id} not found"))?;
        Ok(Message::DocLoaded(DocDetail {
            id: doc.id,
            filename: doc.filename,
            content_type: doc.content_type,
            content: doc.content,
        }))
    })
    .await?
}

/// Delete a document by id, returning [`Message::DocDeleted`].
pub async fn delete_doc(id: i64) -> Result<Message> {
    task::spawn_blocking(move || {
        let db = Database::open()?;
        let store = DocumentStore::new(&db);
        if !store.delete(id)? {
            return Err(anyhow!("Document {id} not found"));
        }
        Ok(Message::DocDeleted { id })
    })
    .await?
}
