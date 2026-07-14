//! Search pane (blueprint §4 — `DocumentStore::search`).
//!
//! Phase 2a stub: owns the state shape and the [`Pane`] wiring; renders a
//! placeholder. A search-pane agent fills `on_key`/`apply`/`render` here and the
//! body of `service::search::run_search`.

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

use super::super::action::{Action, Message, SearchHit};
use super::super::theme::Theme;
use super::pane::{Ctx, Pane, coming_soon};

/// Search screen state.
#[derive(Default)]
#[allow(dead_code)]
pub struct SearchState {
    /// The query being composed / last run.
    pub query: String,
    /// Ranked hits from the last completed search.
    pub hits: Vec<SearchHit>,
    /// Selected hit index.
    pub selected: usize,
    /// Whether the query input is capturing keys.
    pub editing: bool,
}

impl SearchState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Pane for SearchState {
    fn on_key(&mut self, _key: KeyEvent, _ctx: &Ctx) -> Option<Action> {
        // TODO(search-pane): compose the query, and on Enter return
        // `Some(Action::RunSearch { query: self.query.clone() })`.
        None
    }

    fn apply(&mut self, msg: &Message) {
        if let Message::SearchResults { query, hits } = msg {
            self.query = query.clone();
            self.hits = hits.clone();
            self.selected = 0;
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, _focused: bool) {
        coming_soon(
            frame,
            area,
            theme,
            "Search",
            "Full-text search across your library.",
        );
    }

    fn wants_input(&self) -> bool {
        self.editing
    }
}
