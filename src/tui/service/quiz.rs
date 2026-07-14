//! Quiz pane service (blueprint §4).
//!
//! Real implementation generates questions from the current bucket
//! (`get_document_context_pub` → `GroqClient::chat` → `parse_quiz_questions`)
//! and grades answers with `StudyStore::update_after_review` (SM-2). Phase 2a is
//! a stub.

use anyhow::Result;

use crate::tui::action::{Message, ToastLevel};

/// Generate a quiz of `count` questions as [`Message::QuizGenerated`].
///
/// TODO(quiz-pane): build context, call the model, parse questions, map to
/// `StudyCard`s, return `Message::QuizGenerated(cards)`.
pub async fn start_quiz(count: usize) -> Result<Message> {
    let _ = count;
    Ok(Message::Toast {
        level: ToastLevel::Warn,
        text: "Quiz not implemented yet".to_string(),
    })
}

/// Grade a quiz answer (SM-2 quality 0–5) as [`Message::QuizGraded`].
///
/// TODO(quiz-pane): `spawn_blocking` → `StudyStore::update_after_review(id,
/// quality)` → `Message::QuizGraded { id }`.
pub async fn grade_quiz(id: i64, quality: u8) -> Result<Message> {
    let _ = (id, quality);
    Ok(Message::Toast {
        level: ToastLevel::Warn,
        text: "Quiz grading not implemented yet".to_string(),
    })
}
