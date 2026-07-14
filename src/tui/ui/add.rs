//! Add pane (blueprint §4 — ingest pipeline, highest-effort pane).
//!
//! A source line (file path / directory / URL) drives an ingest run whose live
//! progress is shown as a gauge, the current unit of work, and a scrolling log
//! of per-source results. All heavy work happens off the render thread in
//! [`crate::tui::service::add::start_ingest`]; this pane only composes the
//! source, dispatches [`Action::StartIngest`], and folds the streamed
//! [`Message::IngestProgress`] / [`Message::IngestFileDone`] /
//! [`Message::IngestComplete`] results into local state.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Gauge, Paragraph};

use super::super::action::{Action, Message};
use super::super::theme::Theme;
use super::pane::{Ctx, Pane};

/// Add / ingest screen state.
#[derive(Default)]
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

    /// Toggle the file/URL interpretation of the source line.
    fn toggle_url(&mut self) {
        self.is_url = !self.is_url;
    }

    /// Dispatch a fresh ingest run for the current source, resetting live state.
    fn start(&mut self) -> Option<Action> {
        let source = self.source.trim().to_string();
        if source.is_empty() || self.running {
            return None;
        }
        // Auto-detect URLs so the toggle need not be set explicitly.
        if source.starts_with("http://") || source.starts_with("https://") {
            self.is_url = true;
        }
        self.editing = false;
        self.running = true;
        self.done = 0;
        self.total = 0;
        self.current = "Starting…".to_string();
        self.log.clear();
        Some(Action::StartIngest {
            source,
            is_url: self.is_url,
        })
    }
}

impl Pane for AddState {
    fn on_key(&mut self, key: KeyEvent, _ctx: &Ctx) -> Option<Action> {
        if self.editing {
            // While editing, every key is routed here (see `Pane::wants_input`).
            match key.code {
                KeyCode::Esc => {
                    self.editing = false;
                }
                KeyCode::Enter => return self.start(),
                KeyCode::Tab => self.toggle_url(),
                KeyCode::Backspace => {
                    self.source.pop();
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.source.clear();
                }
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    self.source.push(c);
                }
                _ => {}
            }
            return None;
        }

        // Normal mode: global nav keys (Tab/Esc/digits/q/?) are consumed by
        // `app.rs` before reaching us, so only the rest arrive here.
        match key.code {
            KeyCode::Char('i') | KeyCode::Char('e') => {
                if !self.running {
                    self.editing = true;
                }
            }
            KeyCode::Enter => return self.start(),
            KeyCode::Char('u') => self.toggle_url(),
            KeyCode::Char('c') if !self.running => {
                self.log.clear();
                self.current.clear();
                self.done = 0;
                self.total = 0;
            }
            _ => {}
        }
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
                if *chunks > 0 {
                    self.log.push(format!("+ {filename}  ·  {chunks} chunks"));
                } else {
                    // Service pre-formats skip/error lines (glyph-prefixed).
                    self.log.push(filename.clone());
                }
            }
            Message::IngestComplete { added, skipped } => {
                self.running = false;
                self.current.clear();
                if self.total > 0 {
                    self.done = self.total;
                }
                self.log
                    .push(format!("✓ Done  ·  {added} added, {skipped} skipped"));
            }
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, focused: bool) {
        let [input_a, gauge_a, current_a, log_a, help_a] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .areas(area);

        self.render_input(frame, input_a, theme);
        self.render_gauge(frame, gauge_a, theme);
        self.render_current(frame, current_a, theme);
        self.render_log(frame, log_a, theme, focused);
        self.render_help(frame, help_a, theme);
    }

    fn wants_input(&self) -> bool {
        self.editing
    }
}

impl AddState {
    fn render_input(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let tag = if self.is_url { "URL" } else { "PATH" };
        let title = if self.editing {
            format!(" Source · {tag} (Enter to ingest, Tab toggles URL) ")
        } else {
            format!(" Source · {tag} (press i to edit) ")
        };
        let block = Block::bordered()
            .title(title)
            .title_style(if self.editing {
                theme.title()
            } else {
                theme.dim()
            })
            .border_style(theme.border(self.editing));

        let mut content = if self.source.is_empty() && !self.editing {
            String::from("Enter a file path, directory, or URL…")
        } else {
            self.source.clone()
        };
        let style = if self.source.is_empty() && !self.editing {
            theme.dim()
        } else {
            theme.text()
        };
        if self.editing {
            content.push('▌');
        }

        frame.render_widget(
            Paragraph::new(Span::styled(content, style)).block(block),
            area,
        );
    }

    fn render_gauge(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let ratio = if self.total == 0 {
            0.0
        } else {
            (self.done as f64 / self.total as f64).clamp(0.0, 1.0)
        };
        let label = if self.running && self.total == 0 {
            "starting…".to_string()
        } else if self.total == 0 {
            "idle".to_string()
        } else {
            format!("{}/{}  ({:.0}%)", self.done, self.total, ratio * 100.0)
        };

        let block = Block::bordered()
            .title(" Progress ")
            .title_style(theme.title())
            .border_style(theme.border(self.running));

        let gauge = Gauge::default()
            .block(block)
            .gauge_style(theme.accent())
            .ratio(ratio)
            .label(Span::styled(label, theme.text()));

        frame.render_widget(gauge, area);
    }

    fn render_current(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let line = if self.current.is_empty() {
            Line::from(Span::styled("  ", theme.dim()))
        } else {
            Line::from(vec![
                Span::styled("  ▸ ", theme.accent()),
                Span::styled(self.current.clone(), theme.text()),
            ])
        };
        frame.render_widget(Paragraph::new(line), area);
    }

    fn render_log(&self, frame: &mut Frame, area: Rect, theme: &Theme, focused: bool) {
        let block = Block::bordered()
            .title(" Activity ")
            .title_style(theme.title())
            .border_style(theme.border(focused));
        let inner_h = block.inner(area).height.max(1) as usize;

        let mut lines: Vec<Line> = Vec::new();
        if self.log.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Nothing ingested yet.",
                theme.dim(),
            )));
            lines.push(Line::from(Span::styled(
                "  Press i to enter a path or URL, then Enter to ingest.",
                theme.dim(),
            )));
        } else {
            // Auto-follow the tail so the newest results stay visible.
            let start = self.log.len().saturating_sub(inner_h);
            for entry in &self.log[start..] {
                lines.push(log_line(entry, theme));
            }
        }

        frame.render_widget(Paragraph::new(lines).block(block), area);
    }

    fn render_help(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let help = if self.editing {
            "type source · Tab: file/URL · Ctrl+U: clear · Enter: ingest · Esc: cancel"
        } else if self.running {
            "ingesting… · results stream into Activity below"
        } else {
            "i/e: edit · u: file/URL · Enter: ingest · c: clear log"
        };
        frame.render_widget(
            Paragraph::new(Span::styled(format!("  {help}"), theme.dim())),
            area,
        );
    }
}

/// Style one activity-log entry by its leading status glyph.
fn log_line<'a>(entry: &'a str, theme: &Theme) -> Line<'a> {
    let style = match entry.chars().next() {
        Some('+') | Some('✓') => theme.accent(),
        Some('⊘') => theme.dim(),
        Some('✗') => theme.toast(super::super::action::ToastLevel::Error),
        _ => theme.text(),
    };
    Line::from(Span::styled(format!("  {entry}"), style))
}
