//! Study pane (blueprint §4 — `commands/generate.rs`).
//!
//! A tool picker (5 generation kinds), a topic input line, and a
//! save-parsed-items toggle, above a live streamed output pane. Tokens stream in
//! via [`Message::StudyToken`] (rendered as plain wrapped text — no markdown
//! crate yet); the final [`Message::StudyDone`] swaps the live buffer for the
//! finished content.
//!
//! Keys: `↑`/`↓` pick the kind, `i`/`e` edit the topic, `t` toggle save-items,
//! `Enter` generate, `Esc` leave the topic editor.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Paragraph};

use super::super::action::{Action, Message};
use super::super::theme::Theme;
use super::pane::{Ctx, Pane};

/// The five generation kinds as `(action id, display label)`. The `id` is what
/// travels in [`Action::GenerateStudy`] and what `service::study` matches on.
const KINDS: [(&str, &str); 5] = [
    ("study-guide", "Study Guide"),
    ("flashcards", "Flashcards"),
    ("quiz", "Quiz"),
    ("summary", "Summary"),
    ("homework", "Homework"),
];

/// Study screen state.
#[derive(Default)]
#[allow(dead_code)]
pub struct StudyState {
    /// The kind of material to generate (one of [`KINDS`]'s ids).
    pub kind: String,
    /// The topic prompt.
    pub topic: String,
    /// Live token buffer while generating.
    pub stream_buf: String,
    /// Final rendered content once generation completes.
    pub content: String,
    /// Whether generation is in flight.
    pub generating: bool,
    /// Whether to persist parsed flashcards/quiz items.
    pub save_items: bool,
    /// Whether the topic input is capturing keys.
    pub editing: bool,
}

impl StudyState {
    pub fn new() -> Self {
        Self {
            kind: KINDS[0].0.to_string(),
            ..Self::default()
        }
    }

    /// Index of the currently selected kind in [`KINDS`] (defaults to 0).
    fn kind_index(&self) -> usize {
        KINDS
            .iter()
            .position(|(id, _)| *id == self.kind)
            .unwrap_or(0)
    }

    /// Move the kind selection by `delta` (wrapping).
    fn cycle_kind(&mut self, forward: bool) {
        let len = KINDS.len();
        let cur = self.kind_index();
        let next = if forward {
            (cur + 1) % len
        } else {
            (cur + len - 1) % len
        };
        self.kind = KINDS[next].0.to_string();
    }

    /// Whether the current kind produces persistable Q/A pairs.
    fn kind_saves(&self) -> bool {
        matches!(self.kind.as_str(), "flashcards" | "quiz")
    }

    /// Begin a generation run: clear prior output and mark in-flight.
    fn start_generation(&mut self) -> Option<Action> {
        self.content.clear();
        self.stream_buf.clear();
        self.generating = true;
        Some(Action::GenerateStudy {
            kind: self.kind.clone(),
            topic: self.topic.clone(),
            save_items: self.save_items,
        })
    }
}

impl Pane for StudyState {
    fn on_key(&mut self, key: KeyEvent, _ctx: &Ctx) -> Option<Action> {
        if self.editing {
            match key.code {
                KeyCode::Esc => {
                    self.editing = false;
                }
                KeyCode::Enter => {
                    self.editing = false;
                    if !self.generating {
                        return self.start_generation();
                    }
                }
                KeyCode::Backspace => {
                    self.topic.pop();
                }
                KeyCode::Char(c) => {
                    self.topic.push(c);
                }
                _ => {}
            }
            return None;
        }

        match key.code {
            KeyCode::Up => self.cycle_kind(false),
            KeyCode::Down => self.cycle_kind(true),
            KeyCode::Char('i') | KeyCode::Char('e') => self.editing = true,
            KeyCode::Char('t') => self.save_items = !self.save_items,
            KeyCode::Enter if !self.generating => return self.start_generation(),
            _ => {}
        }
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

    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, focused: bool) {
        let [controls, output] =
            Layout::vertical([Constraint::Length(8), Constraint::Min(3)]).areas(area);
        self.render_controls(frame, controls, theme, focused);
        self.render_output(frame, output, theme, focused);
    }

    fn wants_input(&self) -> bool {
        self.editing
    }
}

impl StudyState {
    fn render_controls(&self, frame: &mut Frame, area: Rect, theme: &Theme, focused: bool) {
        let block = Block::bordered()
            .title(" Study — Generate ")
            .title_style(theme.title())
            .border_style(theme.border(focused));

        // Tool picker: highlight the selected kind.
        let selected = self.kind_index();
        let mut tool_spans: Vec<Span> = vec![Span::styled("  Tool  ", theme.dim())];
        for (i, (_, label)) in KINDS.iter().enumerate() {
            let style = if i == selected {
                theme.title().add_modifier(Modifier::REVERSED)
            } else {
                theme.dim()
            };
            tool_spans.push(Span::styled(format!(" {label} "), style));
        }

        // Topic line.
        let topic_display = if self.editing {
            format!("{}▌", self.topic)
        } else if self.topic.is_empty() {
            "(all materials)".to_string()
        } else {
            self.topic.clone()
        };
        let topic_style = if self.editing {
            theme.text()
        } else {
            theme.dim()
        };
        let topic_line = Line::from(vec![
            Span::styled("  Topic ", theme.dim()),
            Span::styled(" ", theme.dim()),
            Span::styled(topic_display, topic_style),
        ]);

        // Save-items toggle (only meaningful for flashcards/quiz).
        let toggle_state = if self.save_items { "on" } else { "off" };
        let toggle_style = if self.save_items {
            ok_style(theme)
        } else {
            theme.dim()
        };
        let mut save_spans: Vec<Span> = vec![
            Span::styled("  Save  ", theme.dim()),
            Span::styled(format!(" [{toggle_state}] "), toggle_style),
            Span::styled("parsed Q/A items", theme.dim()),
        ];
        if self.save_items && !self.kind_saves() {
            save_spans.push(Span::styled("  (flashcards/quiz only)", theme.dim()));
        }

        let status = if self.generating {
            Span::styled("  Generating…", theme.accent())
        } else {
            Span::styled(
                "  ↑/↓ tool · i edit topic · t toggle save · Enter generate · Esc cancel",
                theme.dim(),
            )
        };

        let lines = vec![
            Line::from(""),
            Line::from(tool_spans),
            Line::from(""),
            topic_line,
            Line::from(save_spans),
            Line::from(status),
        ];

        frame.render_widget(Paragraph::new(Text::from(lines)).block(block), area);
    }

    fn render_output(&self, frame: &mut Frame, area: Rect, theme: &Theme, focused: bool) {
        let title = if self.generating {
            " Output · streaming "
        } else if !self.content.is_empty() {
            " Output "
        } else {
            " Output · empty "
        };
        let block = Block::bordered()
            .title(title)
            .title_style(theme.title())
            .border_style(theme.border(focused));
        let inner = block.inner(area);
        let width = inner.width.max(1) as usize;
        let height = inner.height.max(1) as usize;

        let mut lines: Vec<Line> = Vec::new();

        if self.generating || !self.stream_buf.is_empty() {
            let body = format!("{}▌", self.stream_buf);
            push_wrapped(&mut lines, &body, width, theme);
        } else if !self.content.is_empty() {
            push_wrapped(&mut lines, &self.content, width, theme);
        } else {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Pick a tool, set a topic, and press Enter to generate.",
                theme.dim(),
            )));
            lines.push(Line::from(Span::styled(
                "  Generated flashcards & quizzes can be saved for spaced review.",
                theme.dim(),
            )));
        }

        // Auto-follow the tail so streaming stays in view.
        let total = lines.len();
        let start = total.saturating_sub(height);
        let visible: Vec<Line> = lines[start..total].to_vec();

        frame.render_widget(Paragraph::new(Text::from(visible)).block(block), area);
    }
}

/// Success/ok accent style (matches the theme's ok slot).
fn ok_style(theme: &Theme) -> ratatui::style::Style {
    theme.toast(super::super::action::ToastLevel::Success)
}

/// Push `content` into `lines`, wrapping each source line to `width` columns.
fn push_wrapped(lines: &mut Vec<Line<'static>>, content: &str, width: usize, theme: &Theme) {
    for raw in content.split('\n') {
        for wrapped in wrap(raw, width) {
            lines.push(Line::from(Span::styled(wrapped, theme.text())));
        }
    }
}

/// Simple greedy word-wrap to `width` columns (char count, not grapheme-aware).
fn wrap(text: &str, width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let width = width.max(1);
    let mut out: Vec<String> = Vec::new();
    let mut line = String::new();
    let mut line_len = 0usize;

    for word in text.split(' ') {
        let wlen = word.chars().count();
        if wlen > width {
            if line_len > 0 {
                out.push(std::mem::take(&mut line));
                line_len = 0;
            }
            let mut chunk = String::new();
            for ch in word.chars() {
                chunk.push(ch);
                if chunk.chars().count() == width {
                    out.push(std::mem::take(&mut chunk));
                }
            }
            if !chunk.is_empty() {
                line = chunk;
                line_len = line.chars().count();
            }
            continue;
        }

        let extra = if line_len == 0 { wlen } else { wlen + 1 };
        if line_len + extra > width {
            out.push(std::mem::take(&mut line));
            line.push_str(word);
            line_len = wlen;
        } else {
            if line_len > 0 {
                line.push(' ');
                line_len += 1;
            }
            line.push_str(word);
            line_len += wlen;
        }
    }
    out.push(line);
    out
}
