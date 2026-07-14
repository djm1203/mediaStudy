//! Config pane (blueprint §4 — `Config` + `GroqClient::MODELS`).
//!
//! Phase 2a stub: owns the state shape and the [`Pane`] wiring; renders a
//! placeholder. A config-pane agent fills this and `service::config::save_config`
//! (loading is already real).

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

use super::super::action::{Action, ConfigData, Message};
use super::super::theme::Theme;
use super::pane::{Ctx, Pane, coming_soon};

/// Config screen state.
#[derive(Default)]
#[allow(dead_code)]
pub struct ConfigState {
    /// The loaded configuration snapshot.
    pub data: Option<ConfigData>,
    /// Highlighted model in the selector.
    pub selected_model: usize,
    /// API-key input buffer (masked on render).
    pub api_key_input: String,
    /// Whether the API-key input is capturing keys.
    pub editing: bool,
}

impl ConfigState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Pane for ConfigState {
    fn on_key(&mut self, _key: KeyEvent, _ctx: &Ctx) -> Option<Action> {
        // TODO(config-pane): edit the API key / pick a model, then
        // `Action::SaveConfig { api_key, model }`.
        None
    }

    fn apply(&mut self, msg: &Message) {
        if let Message::ConfigLoaded(data) = msg {
            // Preselect the currently active model in the list.
            self.selected_model = data
                .models
                .iter()
                .position(|(id, _)| *id == data.model)
                .unwrap_or(0);
            self.data = Some(data.clone());
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, _focused: bool) {
        coming_soon(
            frame,
            area,
            theme,
            "Config",
            "Set your API key and default model.",
        );
    }

    fn wants_input(&self) -> bool {
        self.editing
    }
}
