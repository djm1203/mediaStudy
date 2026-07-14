//! Config pane (blueprint §4 — multi-provider, E2-core).
//!
//! Lets the operator pick an LLM **provider** (Groq / OpenAI / Anthropic /
//! Ollama), see/set that provider's **API key** (or, for Ollama, its **URL** —
//! Ollama needs no key), pick a **model** from `provider::suggested_models`
//! (or type a custom id), and **save**. Provider/model changes are staged
//! locally and only persist on save.
//!
//! The API key is never rendered in plaintext — only a masked bullet count — and
//! is only ever handed to the worker via `Action::SaveConfig`.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use tokio::sync::mpsc::UnboundedSender;

use super::super::action::{Action, ConfigData, Message};
use super::super::theme::Theme;
use super::pane::{Ctx, Pane, coming_soon};
use crate::llm::provider::{self, ProviderKind};

/// The providers offered in the selector, in display order.
const PROVIDERS: [ProviderKind; 4] = [
    ProviderKind::Groq,
    ProviderKind::OpenAi,
    ProviderKind::Anthropic,
    ProviderKind::Ollama,
];

/// Human-readable label for a provider chip.
fn provider_label(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Groq => "Groq",
        ProviderKind::OpenAi => "OpenAI",
        ProviderKind::Anthropic => "Anthropic",
        ProviderKind::Ollama => "Ollama",
    }
}

/// What the input buffer is currently capturing.
#[derive(Default, PartialEq, Eq)]
enum EditMode {
    #[default]
    None,
    /// Editing the selected provider's API key (masked).
    Key,
    /// Editing the Ollama server URL.
    Url,
    /// Typing a custom model id.
    Model,
}

/// Config screen state.
#[derive(Default)]
pub struct ConfigState {
    /// The loaded configuration snapshot.
    data: Option<ConfigData>,
    /// Selected provider (index into [`PROVIDERS`]).
    selected_provider: usize,
    /// Highlighted model in the selected provider's list.
    selected_model: usize,
    /// Current edit mode.
    edit: EditMode,
    /// Active edit buffer (masked on render only for [`EditMode::Key`]).
    input: String,
    /// Pending, unsaved API key for the selected provider.
    pending_key: Option<String>,
    /// Pending, unsaved Ollama URL.
    pending_url: Option<String>,
    /// Pending, unsaved custom model id (overrides the list selection).
    pending_model: Option<String>,
    /// Cached action sender (only `on_key` gets a [`Ctx`]; `apply` needs this to
    /// refresh the library + reload the pane after a save completes).
    action_tx: Option<UnboundedSender<Action>>,
}

impl ConfigState {
    pub fn new() -> Self {
        Self::default()
    }

    /// The provider currently selected in the UI (not necessarily saved).
    fn selected_kind(&self) -> ProviderKind {
        PROVIDERS[self.selected_provider]
    }

    /// Suggested models for the selected provider.
    fn models(&self) -> &'static [(&'static str, &'static str, usize)] {
        provider::suggested_models(self.selected_kind())
    }

    /// Whether a usable key exists for `kind` (from the loaded snapshot).
    fn key_set_for(&self, kind: ProviderKind) -> bool {
        self.data.as_ref().is_some_and(|d| match kind {
            ProviderKind::Groq => d.groq_key_set,
            ProviderKind::OpenAi => d.openai_key_set,
            ProviderKind::Anthropic => d.anthropic_key_set,
            ProviderKind::Ollama => true,
        })
    }

    /// The model id that a save would persist: a typed custom id, else the
    /// highlighted list entry.
    fn selected_model_id(&self) -> Option<String> {
        if let Some(m) = &self.pending_model {
            return Some(m.clone());
        }
        self.models()
            .get(self.selected_model)
            .map(|(id, _, _)| (*id).to_string())
    }

    /// The current Ollama URL to show/edit (pending, else loaded).
    fn ollama_url(&self) -> String {
        self.pending_url
            .clone()
            .or_else(|| self.data.as_ref().map(|d| d.ollama_url.clone()))
            .unwrap_or_default()
    }

    /// Preselect the loaded model in the current provider's list, else the top.
    fn preselect_model(&self) -> usize {
        let target = self.data.as_ref().map_or("", |d| d.model.as_str());
        self.models()
            .iter()
            .position(|(id, _, _)| *id == target)
            .unwrap_or(0)
    }

    /// Switch the selected provider, resetting the staged model/key/url state.
    fn set_provider(&mut self, idx: usize) {
        self.selected_provider = idx % PROVIDERS.len();
        self.edit = EditMode::None;
        self.input.clear();
        self.pending_key = None;
        self.pending_model = None;
        self.pending_url = None;
        self.selected_model = self.preselect_model();
    }

    /// Commit the active edit buffer into the matching pending slot.
    fn commit_edit(&mut self) {
        let buf = std::mem::take(&mut self.input);
        match self.edit {
            EditMode::Key => {
                if !buf.is_empty() {
                    self.pending_key = Some(buf);
                }
            }
            EditMode::Url => {
                if !buf.is_empty() {
                    self.pending_url = Some(buf);
                }
            }
            EditMode::Model => {
                if !buf.is_empty() {
                    self.pending_model = Some(buf);
                }
            }
            EditMode::None => {}
        }
        self.edit = EditMode::None;
    }

    /// Build the save action for the current selection and optimistically fold
    /// it into the local snapshot for a snappy redraw.
    fn build_save(&mut self) -> Action {
        let kind = self.selected_kind();
        let provider = kind.to_string();
        let is_ollama = kind == ProviderKind::Ollama;

        let api_key = if is_ollama {
            None
        } else {
            self.pending_key.clone()
        };
        let ollama_url = if is_ollama {
            self.pending_url.clone()
        } else {
            None
        };
        let model = self.selected_model_id();

        // Optimistic local update; the follow-up LoadConfig reconciles.
        if let Some(d) = self.data.as_mut() {
            d.provider = provider.clone();
            if let Some(m) = &model {
                d.model = m.clone();
            }
            if let Some(u) = &ollama_url {
                d.ollama_url = u.clone();
            }
            if api_key.is_some() {
                match kind {
                    ProviderKind::Groq => d.groq_key_set = true,
                    ProviderKind::OpenAi => d.openai_key_set = true,
                    ProviderKind::Anthropic => d.anthropic_key_set = true,
                    ProviderKind::Ollama => {}
                }
            }
        }

        self.pending_key = None;
        self.pending_url = None;
        self.input.clear();
        self.edit = EditMode::None;

        Action::SaveConfig {
            provider,
            api_key,
            model,
            ollama_url,
        }
    }
}

impl Pane for ConfigState {
    fn on_key(&mut self, key: KeyEvent, ctx: &Ctx) -> Option<Action> {
        self.action_tx = Some(ctx.action_tx.clone());

        if self.edit != EditMode::None {
            match key.code {
                KeyCode::Enter => self.commit_edit(),
                KeyCode::Esc => {
                    self.input.clear();
                    self.edit = EditMode::None;
                }
                KeyCode::Backspace => {
                    self.input.pop();
                }
                KeyCode::Char(c) => self.input.push(c),
                _ => {}
            }
            return None;
        }

        match key.code {
            KeyCode::Right | KeyCode::Char('p') => {
                self.set_provider(self.selected_provider + 1);
                None
            }
            KeyCode::Left => {
                self.set_provider(self.selected_provider + PROVIDERS.len() - 1);
                None
            }
            KeyCode::Char('e') => {
                if self.selected_kind() == ProviderKind::Ollama {
                    self.input = self.ollama_url();
                    self.edit = EditMode::Url;
                } else {
                    self.input.clear();
                    self.edit = EditMode::Key;
                }
                None
            }
            KeyCode::Char('m') => {
                self.input.clear();
                self.edit = EditMode::Model;
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected_model = self.selected_model.saturating_sub(1);
                self.pending_model = None;
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let count = self.models().len();
                if count > 0 && self.selected_model + 1 < count {
                    self.selected_model += 1;
                }
                self.pending_model = None;
                None
            }
            KeyCode::Char('s') => Some(self.build_save()),
            _ => None,
        }
    }

    fn apply(&mut self, msg: &Message) {
        match msg {
            Message::ConfigLoaded(data) => {
                let kind = data
                    .provider
                    .parse::<ProviderKind>()
                    .unwrap_or(ProviderKind::Groq);
                self.selected_provider = PROVIDERS.iter().position(|k| *k == kind).unwrap_or(0);
                self.data = Some(data.clone());
                self.selected_model = self.preselect_model();
                self.pending_key = None;
                self.pending_url = None;
                self.pending_model = None;
                self.input.clear();
                self.edit = EditMode::None;
            }
            Message::ConfigSaved => {
                self.input.clear();
                self.edit = EditMode::None;
                // Refresh Home/sidebar key+model status and reload our snapshot
                // so the key-set flags reflect what was just written.
                if let Some(tx) = &self.action_tx {
                    let _ = tx.send(Action::LoadLibrary);
                    let _ = tx.send(Action::LoadConfig);
                }
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

        let [provider_area, settings_area, models_area, hint_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .areas(inner);

        render_providers(frame, provider_area, theme, self);
        render_settings(frame, settings_area, theme, self, data);
        render_models(frame, models_area, theme, self, focused);
        render_hints(frame, hint_area, theme, self);
    }

    fn wants_input(&self) -> bool {
        self.edit != EditMode::None
    }
}

fn render_providers(frame: &mut Frame, area: Rect, theme: &Theme, state: &ConfigState) {
    let mut spans: Vec<Span> = vec![Span::styled("  Provider: ", theme.text())];
    for (i, kind) in PROVIDERS.iter().enumerate() {
        let selected = i == state.selected_provider;
        let style = if selected {
            theme
                .text()
                .fg(theme.p.accent)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            theme.dim()
        };
        spans.push(Span::styled(format!(" {} ", provider_label(*kind)), style));
        spans.push(Span::raw(" "));
    }
    frame.render_widget(
        Paragraph::new(vec![Line::from(""), Line::from(spans)]),
        area,
    );
}

fn render_settings(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    state: &ConfigState,
    data: &ConfigData,
) {
    let kind = state.selected_kind();

    // Line 1: credential (API key) or Ollama URL.
    let cred_line = if kind == ProviderKind::Ollama {
        if state.edit == EditMode::Url {
            Line::from(vec![
                Span::styled("  Ollama URL: ", theme.text()),
                Span::styled(format!("{}\u{2588}", state.input), theme.accent()),
            ])
        } else {
            Line::from(vec![
                Span::styled("  Ollama URL: ", theme.text()),
                Span::styled(state.ollama_url(), theme.text().fg(theme.p.accent)),
            ])
        }
    } else if state.edit == EditMode::Key {
        let masked: String = "\u{25cf}".repeat(state.input.chars().count().min(48));
        Line::from(vec![
            Span::styled("  API Key: ", theme.text()),
            Span::styled(format!("{masked}\u{2588}"), theme.accent()),
        ])
    } else if state.pending_key.is_some() {
        Line::from(vec![
            Span::styled("  API Key: ", theme.text()),
            Span::styled("pending (unsaved)", theme.text().fg(theme.p.warn)),
        ])
    } else if state.key_set_for(kind) {
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

    // Line 2: the model that a save would persist.
    let model_id = state
        .selected_model_id()
        .unwrap_or_else(|| data.model.clone());
    let custom = state.pending_model.is_some();
    let model_line = Line::from(vec![
        Span::styled("  Model: ", theme.text()),
        Span::styled(model_id, theme.text().fg(theme.p.warn)),
        Span::styled(if custom { "  (custom)" } else { "" }, theme.dim()),
    ]);

    frame.render_widget(
        Paragraph::new(vec![Line::from(""), cred_line, model_line]),
        area,
    );
}

fn render_models(frame: &mut Frame, area: Rect, theme: &Theme, state: &ConfigState, focused: bool) {
    let active = state.selected_model_id();
    let editing = state.edit != EditMode::None;

    let mut lines: Vec<Line> = Vec::new();
    if state.edit == EditMode::Model {
        lines.push(Line::from(vec![
            Span::styled("  Custom model: ", theme.title()),
            Span::styled(format!("{}\u{2588}", state.input), theme.accent()),
        ]));
    } else {
        lines.push(Line::from(Span::styled("  Models", theme.title())));
    }

    for (i, (id, desc, _)) in state.models().iter().enumerate() {
        let is_active = active.as_deref() == Some(*id);
        let is_sel = focused && !editing && i == state.selected_model;

        let marker = if is_active { "\u{25cf}" } else { " " };
        let mut style = if is_active {
            theme.text().fg(theme.p.accent).add_modifier(Modifier::BOLD)
        } else {
            theme.text()
        };
        if is_sel {
            style = style.bg(theme.p.surface).add_modifier(Modifier::BOLD);
        }

        lines.push(Line::from(Span::styled(
            format!("  {marker} {id}  \u{2014} {desc}"),
            style,
        )));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_hints(frame: &mut Frame, area: Rect, theme: &Theme, state: &ConfigState) {
    let text = if state.edit != EditMode::None {
        "  Enter commit \u{00b7} Esc cancel"
    } else if state.selected_kind() == ProviderKind::Ollama {
        "  \u{2190}/\u{2192} or p provider \u{00b7} e edit url \u{00b7} \u{2191}/\u{2193} model \u{00b7} m custom \u{00b7} s save"
    } else {
        "  \u{2190}/\u{2192} or p provider \u{00b7} e edit key \u{00b7} \u{2191}/\u{2193} model \u{00b7} m custom \u{00b7} s save"
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(text, theme.dim()))),
        area,
    );
}
