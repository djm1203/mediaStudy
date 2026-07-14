//! Study pane (blueprint §4 — `commands/generate.rs`).
//!
//! Phase 2a stub: owns the state shape and the [`Pane`] wiring; renders a
//! placeholder. A study-pane agent fills this and
//! `service::study::generate_study`.

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

use super::super::action::{Action, Message};
use super::super::theme::Theme;
use super::pane::{Ctx, Pane, coming_soon};

/// Study screen state.
#[derive(Default)]
#[allow(dead_code)]
pub struct StudyState {
    /// The kind of material to generate (notes/summary/flashcards/…).
    pub kind: String,
    /// The topic prompt.
    pub topic: String,
    /// Live token buffer while generating.
    pub stream_buf: String,
    /// Final rendered content once generation completes.
    pub content: String,
    /// Whether generation is in flight.
    pub generating: bool,
    /// Whether to persist parsed flashcards.
    pub save_items: bool,
    /// Whether the topic input is capturing keys.
    pub editing: bool,
}

impl StudyState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Pane for StudyState {
    fn on_key(&mut self, _key: KeyEvent, _ctx: &Ctx) -> Option<Action> {
        // TODO(study-pane): pick a kind + topic, Enter → `Action::GenerateStudy
        // { kind, topic, save_items }`.
        None
    }

    fn apply(&mut self, msg: &Message) {
        match msg {
            Message::StudyToken(tok) => {
                self.generating = true;
                self.stream_buf.push_str(tok);
            }
            Message::StudyDone {
                content,
                saved_items: _,
            } => {
                self.content = content.clone();
                self.stream_buf.clear();
                self.generating = false;
            }
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, _focused: bool) {
        coming_soon(
            frame,
            area,
            theme,
            "Study",
            "Generate notes, summaries, and flashcards.",
        );
    }

    fn wants_input(&self) -> bool {
        self.editing
    }
}
