//! Docs pane (blueprint §4 — `DocumentStore::{list,get,delete}`).
//!
//! Phase 2a stub: owns the state shape and the [`Pane`] wiring; renders a
//! placeholder. A docs-pane agent fills this and the `service::docs::*` bodies.

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

use super::super::action::{Action, DocDetail, DocMeta, Message};
use super::super::theme::Theme;
use super::pane::{Ctx, Pane, coming_soon};

/// Docs screen state.
#[derive(Default)]
#[allow(dead_code)]
pub struct DocsState {
    /// Documents in the current bucket.
    pub docs: Vec<DocMeta>,
    /// Selected row.
    pub selected: usize,
    /// The open document's full content (detail view), if any.
    pub detail: Option<DocDetail>,
    /// Whether a delete-confirm prompt is showing.
    pub confirm_delete: bool,
}

impl DocsState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Pane for DocsState {
    fn on_key(&mut self, _key: KeyEvent, _ctx: &Ctx) -> Option<Action> {
        // TODO(docs-pane): j/k to move, Enter → `Action::LoadDoc { id }`,
        // d + confirm → `Action::DeleteDoc { id }`.
        None
    }

    fn apply(&mut self, msg: &Message) {
        match msg {
            Message::DocsLoaded(docs) => {
                self.docs = docs.clone();
                if self.selected >= self.docs.len() {
                    self.selected = self.docs.len().saturating_sub(1);
                }
            }
            Message::DocLoaded(detail) => self.detail = Some(detail.clone()),
            Message::DocDeleted { id } => {
                self.docs.retain(|d| d.id != *id);
                self.confirm_delete = false;
            }
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, _focused: bool) {
        coming_soon(
            frame,
            area,
            theme,
            "Docs",
            "Browse, open, and delete documents.",
        );
    }
}
