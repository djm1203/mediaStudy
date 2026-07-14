//! Review pane (blueprint §4 — spaced repetition over `StudyStore`).
//!
//! A classic flashcard flow: show the current due card's front, reveal the
//! back on Space/Enter, then self-rate recall with `1`-`5` (SM-2 quality,
//! mirroring `commands/review.rs`'s CLI scale) to grade and advance.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use super::super::action::{Action, Message, StudyCard};
use super::super::theme::Theme;
use super::pane::{Ctx, Pane};

/// Review screen state.
#[derive(Default)]
#[allow(dead_code)]
pub struct ReviewState {
    /// Items due for review.
    pub due: Vec<StudyCard>,
    /// Current item index.
    pub index: usize,
    /// Whether the back of the current card is revealed.
    pub revealed: bool,
}

impl ReviewState {
    pub fn new() -> Self {
        Self::default()
    }

    fn current(&self) -> Option<&StudyCard> {
        self.due.get(self.index)
    }
}

impl Pane for ReviewState {
    fn on_key(&mut self, key: KeyEvent, _ctx: &Ctx) -> Option<Action> {
        let id = self.current()?.id;
        match key.code {
            // First Esc while the answer is showing just hides it again
            // (dropping capture); a second Esc then reaches the global
            // handler and leaves the pane, since `wants_input` goes false.
            KeyCode::Esc if self.revealed => {
                self.revealed = false;
                None
            }
            KeyCode::Enter | KeyCode::Char(' ') if !self.revealed => {
                self.revealed = true;
                None
            }
            KeyCode::Char(c) if self.revealed && ('1'..='5').contains(&c) => {
                let quality = c.to_digit(10).unwrap_or(3) as u8;
                Some(Action::GradeReview { id, quality })
            }
            _ => None,
        }
    }

    fn apply(&mut self, msg: &Message) {
        match msg {
            Message::DueLoaded(due) => {
                self.due = due.clone();
                self.index = 0;
                self.revealed = false;
            }
            Message::ReviewGraded { id: _ } => {
                self.revealed = false;
                if self.index + 1 < self.due.len() {
                    self.index += 1;
                }
            }
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, focused: bool) {
        let total = self.due.len();
        let title = if total == 0 {
            " Review ".to_string()
        } else if self.index >= total {
            " Review · done ".to_string()
        } else {
            format!(" Review · {}/{} ", self.index + 1, total)
        };

        let block = Block::bordered()
            .title(title)
            .title_style(theme.title())
            .border_style(theme.border(focused));
        let inner = block.inner(area);
        let width = inner.width.max(1) as usize;

        let mut lines: Vec<Line> = Vec::new();

        if total == 0 {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Nothing due for review right now.",
                theme.text(),
            )));
            lines.push(Line::from(Span::styled(
                "  Generate flashcards or take a quiz, then check back later.",
                theme.dim(),
            )));
        } else if self.index >= total {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!(
                    "  Session complete — {total} card{} reviewed.",
                    if total == 1 { "" } else { "s" }
                ),
                theme.text().fg(theme.p.ok),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Esc to return home. Re-enter this screen to pull newly due items.",
                theme.dim(),
            )));
        } else {
            let card = &self.due[self.index];
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  ({})", card.item_type),
                theme.dim(),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("  Q:", theme.accent())));
            for wrapped in wrap(&card.front, width.saturating_sub(2)) {
                lines.push(Line::from(Span::styled(
                    format!("  {wrapped}"),
                    theme.text(),
                )));
            }
            lines.push(Line::from(""));

            if self.revealed {
                lines.push(Line::from(Span::styled(
                    "  A:",
                    theme.text().fg(theme.p.ok),
                )));
                for wrapped in wrap(&card.back, width.saturating_sub(2)) {
                    lines.push(Line::from(Span::styled(
                        format!("  {wrapped}"),
                        theme.text(),
                    )));
                }
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  How well did you recall this?",
                    theme.dim(),
                )));
                lines.push(Line::from(Span::styled(
                    "  1 blackout · 2 wrong · 3 hard · 4 good · 5 easy",
                    theme.dim(),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    "  Press Space or Enter to reveal the answer.",
                    theme.dim(),
                )));
            }
        }

        frame.render_widget(Paragraph::new(lines).block(block), area);
    }

    fn wants_input(&self) -> bool {
        self.revealed && self.index < self.due.len()
    }
}

/// Simple greedy word-wrap to `width` columns (char count, not grapheme-aware).
/// Mirrors `ui/chat.rs`'s wrap so long card text doesn't overrun the pane.
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
