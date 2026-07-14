//! Review pane (blueprint §4 — spaced repetition over `StudyStore`).
//!
//! Phase 2a stub: owns the state shape and the [`Pane`] wiring; renders a
//! placeholder. A review-pane agent fills this and the `service::review::*`
//! bodies.

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

use super::super::action::{Action, Message, StudyCard};
use super::super::theme::Theme;
use super::pane::{Ctx, Pane, coming_soon};

/// Review screen state.
#[derive(Default)]
#[allow(dead_code)]
pub struct ReviewState {
    /// Items due for review.
    pub due: Vec<StudyCard>,
    /// Current item index.
    pub index: usize,
    /// Whether the back of the current card is revealed.
    pub revealed: bool,
}

impl ReviewState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Pane for ReviewState {
    fn on_key(&mut self, _key: KeyEvent, _ctx: &Ctx) -> Option<Action> {
        // TODO(review-pane): space to reveal; 0–5 → `Action::GradeReview { id,
        // quality }`.
        None
    }

    fn apply(&mut self, msg: &Message) {
        match msg {
            Message::DueLoaded(due) => {
                self.due = due.clone();
                self.index = 0;
                self.revealed = false;
            }
            Message::ReviewGraded { id: _ } => {
                self.revealed = false;
                if self.index + 1 < self.due.len() {
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
            "Review",
            "Spaced-repetition review of due study items.",
        );
    }
}
