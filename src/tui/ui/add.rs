//! Add pane (blueprint §4 — ingest pipeline, highest-effort pane).
//!
//! Phase 2a stub: owns the state shape and the [`Pane`] wiring; renders a
//! placeholder. An add-pane agent fills this and `service::add::start_ingest`.

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

use super::super::action::{Action, Message};
use super::super::theme::Theme;
use super::pane::{Ctx, Pane, coming_soon};

/// Add / ingest screen state.
#[derive(Default)]
#[allow(dead_code)]
pub struct AddState {
    /// The path or URL being entered.
    pub source: String,
    /// Whether `source` is a URL (vs. a filesystem path).
    pub is_url: bool,
    /// Whether an ingest run is in progress.
    pub running: bool,
    /// Files/units completed and total for the progress bar.
    pub done: usize,
    pub total: usize,
    /// The file currently being processed.
    pub current: String,
    /// Rolling activity log.
    pub log: Vec<String>,
    /// Whether the source input is capturing keys.
    pub editing: bool,
}

impl AddState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Pane for AddState {
    fn on_key(&mut self, _key: KeyEvent, _ctx: &Ctx) -> Option<Action> {
        // TODO(add-pane): compose a path/URL, Enter → `Action::StartIngest {
        // source, is_url }`.
        None
    }

    fn apply(&mut self, msg: &Message) {
        match msg {
            Message::IngestProgress {
                done,
                total,
                current,
            } => {
                self.running = true;
                self.done = *done;
                self.total = *total;
                self.current = current.clone();
            }
            Message::IngestFileDone { filename, chunks } => {
                self.log.push(format!("{filename} — {chunks} chunks"));
            }
            Message::IngestComplete { added, skipped } => {
                self.running = false;
                self.log
                    .push(format!("Done: {added} added, {skipped} skipped"));
            }
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, _focused: bool) {
        coming_soon(
            frame,
            area,
            theme,
            "Add",
            "Ingest files or URLs into the current bucket.",
        );
    }

    fn wants_input(&self) -> bool {
        self.editing
    }
}
