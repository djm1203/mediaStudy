//! Review pane service (blueprint §4 — spaced repetition).
//!
//! Calls `StudyStore::{get_due,update_after_review}` (SM-2) inside
//! `tokio::task::spawn_blocking`, each opening its own [`Database`].

use anyhow::Result;
use tokio::task;

use crate::storage::{Database, StudyStore};
use crate::tui::action::{Message, StudyCard};

/// Cap on how many due items we pull into the pane at once. Mirrors the
/// `librarian review` CLI flow (`commands/review.rs`), which pulls 50.
const DUE_LIMIT: usize = 50;

/// Load study items due for review as [`Message::DueLoaded`].
pub async fn load_due() -> Result<Message> {
    task::spawn_blocking(|| {
        let db = Database::open()?;
        let store = StudyStore::new(&db);
        let cards = store
            .get_due(DUE_LIMIT)?
            .into_iter()
            .map(|item| StudyCard {
                id: item.id,
                item_type: item.item_type,
                front: item.front,
                back: item.back,
            })
            .collect();
        Ok(Message::DueLoaded(cards))
    })
    .await?
}

/// Grade a review (SM-2 quality 0–5) as [`Message::ReviewGraded`].
pub async fn grade_review(id: i64, quality: u8) -> Result<Message> {
    task::spawn_blocking(move || {
        let db = Database::open()?;
        let store = StudyStore::new(&db);
        store.update_after_review(id, quality)?;
        Ok(Message::ReviewGraded { id })
    })
    .await?
}
