//! Review pane service (blueprint §4 — spaced repetition).
//!
//! Real implementation calls `StudyStore::{count_due,get_due,update_after_review}`
//! (SM-2) inside `tokio::task::spawn_blocking`. Phase 2a is a stub.

use anyhow::Result;

use crate::tui::action::{Message, ToastLevel};

/// Load study items due for review as [`Message::DueLoaded`].
///
/// TODO(review-pane): `spawn_blocking` → `StudyStore::get_due(limit)` → map to
/// `StudyCard`s → `Message::DueLoaded(cards)`.
pub async fn load_due() -> Result<Message> {
    Ok(Message::Toast {
        level: ToastLevel::Warn,
        text: "Review not implemented yet".to_string(),
    })
}

/// Grade a review (SM-2 quality 0–5) as [`Message::ReviewGraded`].
///
/// TODO(review-pane): `spawn_blocking` → `StudyStore::update_after_review(id,
/// quality)` → `Message::ReviewGraded { id }`.
pub async fn grade_review(id: i64, quality: u8) -> Result<Message> {
    let _ = (id, quality);
    Ok(Message::Toast {
        level: ToastLevel::Warn,
        text: "Review grading not implemented yet".to_string(),
    })
}
