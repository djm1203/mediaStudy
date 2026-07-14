//! Docs pane (blueprint §4 — `DocumentStore::{list,get,delete}`).
//!
//! Left column: the document list (filename, type, created). Right column: the
//! full content of the selected document, loaded on demand. `d` opens an
//! in-pane delete-confirmation prompt (`y` confirms, `n`/Esc cancels).

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};
use tokio::sync::mpsc::UnboundedSender;

use super::super::action::{Action, DocDetail, DocMeta, Message};
use super::super::theme::Theme;
use super::pane::{Ctx, Pane};

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
    /// Cached action sender (only `on_key` gets a [`Ctx`]; `apply` needs this
    /// to fire a follow-up refresh after a delete completes).
    action_tx: Option<UnboundedSender<Action>>,
}

impl DocsState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Pane for DocsState {
    fn on_key(&mut self, key: KeyEvent, ctx: &Ctx) -> Option<Action> {
        self.action_tx = Some(ctx.action_tx.clone());

        if self.confirm_delete {
            return match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    self.confirm_delete = false;
                    self.docs
                        .get(self.selected)
                        .map(|d| Action::DeleteDoc { id: d.id })
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.confirm_delete = false;
                    None
                }
                _ => None,
            };
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.docs.is_empty() {
                    self.selected = (self.selected + 1).min(self.docs.len() - 1);
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                None
            }
            KeyCode::Enter => self
                .docs
                .get(self.selected)
                .map(|d| Action::LoadDoc { id: d.id }),
            KeyCode::Char('d') => {
                if !self.docs.is_empty() {
                    self.confirm_delete = true;
                }
                None
            }
            KeyCode::Char('r') => Some(Action::LoadDocs),
            _ => None,
        }
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
                if self.selected >= self.docs.len() {
                    self.selected = self.docs.len().saturating_sub(1);
                }
                if self.detail.as_ref().is_some_and(|d| d.id == *id) {
                    self.detail = None;
                }
                self.confirm_delete = false;
                // Re-sync with the store (order/counts) now that a row is gone.
                if let Some(tx) = &self.action_tx {
                    let _ = tx.send(Action::LoadDocs);
                }
            }
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, focused: bool) {
        let [list_area, detail_area] =
            Layout::horizontal([Constraint::Percentage(38), Constraint::Percentage(62)])
                .areas(area);

        self.render_list(frame, list_area, theme, focused);
        self.render_detail(frame, detail_area, theme);

        if self.confirm_delete {
            self.render_confirm(frame, area, theme);
        }
    }

    fn wants_input(&self) -> bool {
        // While the delete prompt is up, capture every key (y/n/Esc) so
        // global bindings (q, 1-9, Esc-to-Home) don't hijack the confirm.
        self.confirm_delete
    }
}

impl DocsState {
    fn render_list(&self, frame: &mut Frame, area: Rect, theme: &Theme, focused: bool) {
        let block = Block::bordered()
            .title(format!(" Docs ({}) ", self.docs.len()))
            .title_style(theme.title())
            .border_style(theme.border(focused));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.docs.is_empty() {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled("  No documents yet.", theme.dim())),
                Line::from(Span::styled("  Press 5 to add materials.", theme.dim())),
            ];
            frame.render_widget(Paragraph::new(lines), inner);
            return;
        }

        let mut lines: Vec<Line> = Vec::new();
        for (i, doc) in self.docs.iter().enumerate() {
            let is_sel = i == self.selected;
            let marker = if is_sel { "> " } else { "  " };
            let name_style = if is_sel {
                Style::default()
                    .fg(theme.p.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                theme.text()
            };
            lines.push(Line::from(Span::styled(
                format!("{marker}{}", doc.filename),
                name_style,
            )));
            let tags = doc.tags.as_deref().unwrap_or("");
            let tags_part = if tags.is_empty() {
                String::new()
            } else {
                format!("  ·  {tags}")
            };
            lines.push(Line::from(Span::styled(
                format!(
                    "    {}  ·  {}  ·  {} chars{tags_part}",
                    doc.content_type, doc.created_at, doc.content_len
                ),
                theme.dim(),
            )));
        }
        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_detail(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let title = match &self.detail {
            Some(d) => format!(" {} ", d.filename),
            None => " Detail ".to_string(),
        };
        let block = Block::bordered()
            .title(title)
            .title_style(theme.title())
            .border_style(theme.border(false));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let Some(detail) = &self.detail else {
            let lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Select a document and press Enter to view it.",
                    theme.dim(),
                )),
            ];
            frame.render_widget(Paragraph::new(lines), inner);
            return;
        };

        let mut lines: Vec<Line> = vec![
            Line::from(Span::styled(detail.content_type.clone(), theme.dim())),
            Line::from(""),
        ];
        for raw in detail.content.lines() {
            lines.push(Line::from(Span::styled(raw.to_string(), theme.text())));
        }

        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    }

    fn render_confirm(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let filename = self
            .docs
            .get(self.selected)
            .map(|d| d.filename.as_str())
            .unwrap_or("this document");

        let prompt = format!("Delete '{filename}'?");
        let width = (prompt.chars().count() as u16 + 8)
            .max(32)
            .min(area.width.saturating_sub(2).max(1));
        let height = 5u16.min(area.height.max(1));
        let popup = centered_rect(width, height, area);

        frame.render_widget(Clear, popup);
        let block = Block::bordered()
            .title(" Confirm Delete ")
            .title_style(
                Style::default()
                    .fg(theme.p.err)
                    .add_modifier(Modifier::BOLD),
            )
            .border_style(Style::default().fg(theme.p.err));
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(prompt, theme.text())),
            Line::from(""),
            Line::from(Span::styled("y = confirm   n / Esc = cancel", theme.dim())),
        ];
        frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
    }
}

/// A small `width x height` box centered within `area`.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}
