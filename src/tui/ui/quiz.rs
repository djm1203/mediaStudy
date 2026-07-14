//! Quiz pane (blueprint §4 — `commands/quiz.rs` + `StudyStore`).
//!
//! Phase 2a stub: owns the state shape and the [`Pane`] wiring; renders a
//! placeholder. A quiz-pane agent fills this and the `service::quiz::*` bodies.

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

use super::super::action::{Action, Message, StudyCard};
use super::super::theme::Theme;
use super::pane::{Ctx, Pane, coming_soon};

/// Quiz screen state.
#[derive(Default)]
#[allow(dead_code)]
pub struct QuizState {
    /// The generated question set.
    pub cards: Vec<StudyCard>,
    /// Current question index.
    pub index: usize,
    /// Whether the answer for the current card is revealed.
    pub revealed: bool,
}

impl QuizState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Pane for QuizState {
    fn on_key(&mut self, _key: KeyEvent, _ctx: &Ctx) -> Option<Action> {
        // TODO(quiz-pane): start → `Action::StartQuiz { count }`; grade →
        // `Action::GradeQuiz { id, quality }`.
        None
    }

    fn apply(&mut self, msg: &Message) {
        match msg {
            Message::QuizGenerated(cards) => {
                self.cards = cards.clone();
                self.index = 0;
                self.revealed = false;
            }
            Message::QuizGraded { id: _ } => {
                self.revealed = false;
                if self.index + 1 < self.cards.len() {
                    self.index += 1;
                }
            }
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, _focused: bool) {
        coming_soon(
            frame,
            area,
            theme,
            "Quiz",
            "Generate and answer quizzes from your materials.",
        );
    }
}
