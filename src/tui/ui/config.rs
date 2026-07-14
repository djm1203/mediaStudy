//! Config pane (blueprint §4 — `Config` + `GroqClient::MODELS`).
//!
//! Shows the current API-key status and default model, lets the operator pick
//! a model from `GroqClient::MODELS`, and edit the Groq API key in place. The
//! key is never rendered in plaintext — only a masked bullet count — and is
//! only ever handed to the worker via `Action::SaveConfig`.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

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

    fn model_count(&self) -> usize {
        self.data.as_ref().map_or(0, |d| d.models.len())
    }
}

impl Pane for ConfigState {
    fn on_key(&mut self, key: KeyEvent, _ctx: &Ctx) -> Option<Action> {
        if self.editing {
            return match key.code {
                // Commit: the buffer already holds the pending key value.
                KeyCode::Enter => {
                    self.editing = false;
                    None
                }
                KeyCode::Esc => {
                    self.editing = false;
                    self.api_key_input.clear();
                    None
                }
                KeyCode::Backspace => {
                    self.api_key_input.pop();
                    None
                }
                KeyCode::Char(c) => {
                    self.api_key_input.push(c);
                    None
                }
                _ => None,
            };
        }

        match key.code {
            KeyCode::Char('e') => {
                self.editing = true;
                self.api_key_input.clear();
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected_model = self.selected_model.saturating_sub(1);
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let count = self.model_count();
                if count > 0 && self.selected_model + 1 < count {
                    self.selected_model += 1;
                }
                None
            }
            KeyCode::Char('s') => {
                let api_key = if self.api_key_input.is_empty() {
                    None
                } else {
                    Some(self.api_key_input.clone())
                };
                let model = self
                    .data
                    .as_ref()
                    .and_then(|d| d.models.get(self.selected_model))
                    .map(|(id, _)| id.clone());

                if api_key.is_none() && model.is_none() {
                    // Nothing pending to persist.
                    return None;
                }

                // Optimistically reflect the pending save locally; the worker
                // does not round-trip a fresh `ConfigLoaded` after saving.
                if let Some(data) = self.data.as_mut() {
                    if api_key.is_some() {
                        data.has_api_key = true;
                    }
                    if let Some(m) = &model {
                        data.model = m.clone();
                    }
                }
                self.api_key_input.clear();

                Some(Action::SaveConfig { api_key, model })
            }
            _ => None,
        }
    }

    fn apply(&mut self, msg: &Message) {
        match msg {
            Message::ConfigLoaded(data) => {
                // Preselect the currently active model in the list.
                self.selected_model = data
                    .models
                    .iter()
                    .position(|(id, _)| *id == data.model)
                    .unwrap_or(0);
                self.data = Some(data.clone());
                self.editing = false;
                self.api_key_input.clear();
            }
            Message::ConfigSaved => {
                self.editing = false;
                self.api_key_input.clear();
            }
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, focused: bool) {
        let Some(data) = &self.data else {
            coming_soon(frame, area, theme, "Config", "Loading configuration…");
            return;
        };

        let block = Block::bordered()
            .title(" Config ")
            .title_style(theme.title())
            .border_style(theme.border(focused));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let [settings_area, models_area, hint_area] = Layout::vertical([
            Constraint::Length(4),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .areas(inner);

        render_settings(frame, settings_area, theme, self, data);
        render_models(frame, models_area, theme, self, data, focused);
        render_hints(frame, hint_area, theme, self);
    }

    fn wants_input(&self) -> bool {
        self.editing
    }
}

fn render_settings(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    state: &ConfigState,
    data: &ConfigData,
) {
    let key_line = if state.editing {
        let masked: String = "\u{25cf}".repeat(state.api_key_input.chars().count().min(48));
        Line::from(vec![
            Span::styled("  API Key: ", theme.text()),
            Span::styled(format!("{masked}\u{2588}"), theme.accent()),
        ])
    } else if data.has_api_key {
        Line::from(vec![
            Span::styled("  API Key: ", theme.text()),
            Span::styled(
                "\u{25cf}\u{25cf}\u{25cf}\u{25cf}\u{25cf}\u{25cf}\u{25cf}\u{25cf} set",
                theme.text().fg(theme.p.ok),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled("  API Key: ", theme.text()),
            Span::styled("not set", theme.text().fg(theme.p.err)),
        ])
    };

    let model_line = Line::from(vec![
        Span::styled("  Default model: ", theme.text()),
        Span::styled(data.model.clone(), theme.text().fg(theme.p.warn)),
    ]);

    frame.render_widget(
        Paragraph::new(vec![Line::from(""), key_line, model_line]),
        area,
    );
}

fn render_models(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    state: &ConfigState,
    data: &ConfigData,
    focused: bool,
) {
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled("  Models", theme.title())));

    for (i, (id, desc)) in data.models.iter().enumerate() {
        let is_current = *id == data.model;
        let is_sel = focused && !state.editing && i == state.selected_model;

        let marker = if is_current { "\u{25cf}" } else { " " };
        let mut style = if is_current {
            theme.text().fg(theme.p.accent).add_modifier(Modifier::BOLD)
        } else {
            theme.text()
        };
        if is_sel {
            style = style.bg(theme.p.surface).add_modifier(Modifier::BOLD);
        }

        lines.push(Line::from(Span::styled(
            format!("  {marker} {id}  — {desc}"),
            style,
        )));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_hints(frame: &mut Frame, area: Rect, theme: &Theme, state: &ConfigState) {
    let text = if state.editing {
        "  Enter commit · Esc cancel"
    } else {
        "  e edit key · \u{2191}/\u{2193} or j/k select model · s save"
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(text, theme.dim()))),
        area,
    );
}
