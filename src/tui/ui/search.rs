//! Search pane (blueprint §4 — `DocumentStore::search`).
//!
//! A query input line plus a ranked results list. `/` or `i` focuses the query
//! for editing; Enter runs the search (`Action::RunSearch`); Esc leaves edit
//! mode. Results (`Message::SearchResults`) are browsable with `j`/`k` or the
//! arrow keys.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use super::super::action::{Action, Message, SearchHit};
use super::super::theme::Theme;
use super::pane::{Ctx, Pane};

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

    fn move_selection(&mut self, delta: isize) {
        if self.hits.is_empty() {
            self.selected = 0;
            return;
        }
        let len = self.hits.len() as isize;
        let next = (self.selected as isize + delta).clamp(0, len - 1);
        self.selected = next as usize;
    }
}

impl Pane for SearchState {
    fn on_key(&mut self, key: KeyEvent, _ctx: &Ctx) -> Option<Action> {
        if self.editing {
            return match key.code {
                KeyCode::Esc => {
                    self.editing = false;
                    None
                }
                KeyCode::Enter => {
                    let query = self.query.trim().to_string();
                    if query.is_empty() {
                        None
                    } else {
                        self.editing = false;
                        Some(Action::RunSearch { query })
                    }
                }
                KeyCode::Backspace => {
                    self.query.pop();
                    None
                }
                KeyCode::Char(c) => {
                    self.query.push(c);
                    None
                }
                _ => None,
            };
        }

        match key.code {
            KeyCode::Char('/') | KeyCode::Char('i') => self.editing = true,
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            _ => {}
        }
        None
    }

    fn apply(&mut self, msg: &Message) {
        if let Message::SearchResults { query, hits } = msg {
            self.query = query.clone();
            self.hits = hits.clone();
            self.selected = 0;
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, focused: bool) {
        let [input_area, results_area] =
            Layout::vertical([Constraint::Length(3), Constraint::Min(3)]).areas(area);

        render_input(self, frame, input_area, theme);
        render_results(self, frame, results_area, theme, focused);
    }

    fn wants_input(&self) -> bool {
        self.editing
    }
}

fn render_input(state: &SearchState, frame: &mut Frame, area: Rect, theme: &Theme) {
    let title = if state.editing {
        " Search (Enter to run · Esc to stop editing) "
    } else {
        " Search (/ or i to edit) "
    };
    let block = Block::bordered()
        .title(title)
        .title_style(if state.editing {
            theme.title()
        } else {
            theme.dim()
        })
        .border_style(theme.border(state.editing));

    let mut content = state.query.clone();
    if state.editing {
        content.push('▌');
    }
    let style = if state.editing {
        theme.text()
    } else {
        theme.dim()
    };

    frame.render_widget(
        Paragraph::new(Span::styled(content, style)).block(block),
        area,
    );
}

fn render_results(
    state: &SearchState,
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    focused: bool,
) {
    let block = Block::bordered()
        .title(format!(" Results ({}) ", state.hits.len()))
        .title_style(theme.title())
        .border_style(theme.border(focused && !state.editing));
    let inner = block.inner(area);
    let height = inner.height.max(1) as usize;

    if state.hits.is_empty() {
        let mut lines = vec![Line::from("")];
        if state.query.is_empty() {
            lines.push(Line::from(Span::styled(
                "  Press / or i, type a query, then Enter to search your library.",
                theme.dim(),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                format!("  No results for \"{}\".", state.query),
                theme.dim(),
            )));
        }
        frame.render_widget(Paragraph::new(lines).block(block), area);
        return;
    }

    // Each hit renders as a 3-line block: header, snippet, spacer.
    const ROWS_PER_HIT: usize = 3;
    let mut lines: Vec<Line> = Vec::with_capacity(state.hits.len() * ROWS_PER_HIT);
    for (i, hit) in state.hits.iter().enumerate() {
        let selected = i == state.selected;
        let marker = if selected { "▶" } else { " " };
        let header_style = if selected {
            theme.title()
        } else {
            theme.text()
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {marker} #{} ", hit.id), header_style),
            Span::styled(hit.filename.clone(), header_style),
            Span::styled(format!("  [{}]", hit.content_type), theme.dim()),
        ]));
        lines.push(Line::from(Span::styled(
            format!("     {}", hit.snippet),
            theme.dim(),
        )));
        lines.push(Line::from(""));
    }

    // Keep the selected hit's rows in view.
    let total = lines.len();
    let sel_start = state.selected * ROWS_PER_HIT;
    let start = if height >= total || sel_start < height {
        0
    } else {
        (sel_start + ROWS_PER_HIT)
            .saturating_sub(height)
            .min(total - height)
    };
    let end = (start + height).min(total);
    let visible: Vec<Line> = lines[start..end].to_vec();

    frame.render_widget(Paragraph::new(visible).block(block), area);
}
