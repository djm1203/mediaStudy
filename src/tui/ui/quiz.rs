//! Quiz pane (blueprint §4 — `commands/quiz.rs` + `StudyStore`).
//!
//! Flow: a start screen to choose how many questions → a loading view while the
//! model generates → one card per question (reveal the answer, then self-grade
//! with an SM-2 quality) → a final summary. While a question is on screen the
//! pane captures input (`wants_input`) so the digit grade keys don't trigger
//! global nav; `Esc` leaves the quiz.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};

use super::super::action::{Action, Message, StudyCard};
use super::super::theme::Theme;
use super::pane::{Ctx, Pane};

const MIN_COUNT: usize = 1;
const MAX_COUNT: usize = 20;
const DEFAULT_COUNT: usize = 10;

/// Quiz screen state.
#[allow(dead_code)]
pub struct QuizState {
    /// The generated question set.
    pub cards: Vec<StudyCard>,
    /// Current question index.
    pub index: usize,
    /// Whether the answer for the current card is revealed.
    pub revealed: bool,
    /// Chosen question count on the start screen.
    count: usize,
    /// True once a quiz session is underway (past the start screen).
    started: bool,
    /// True while the model is generating the quiz.
    loading: bool,
    /// Number of cards graded so far.
    answered: usize,
    /// Number graded "correct" (SM-2 quality ≥ 3).
    score: usize,
}

impl Default for QuizState {
    fn default() -> Self {
        Self {
            cards: Vec::new(),
            index: 0,
            revealed: false,
            count: DEFAULT_COUNT,
            started: false,
            loading: false,
            answered: 0,
            score: 0,
        }
    }
}

impl QuizState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a question card is currently on screen (drives `wants_input`).
    fn in_question(&self) -> bool {
        self.started && !self.loading && !self.cards.is_empty() && self.answered < self.cards.len()
    }

    /// Whether the session has finished (every card graded).
    fn finished(&self) -> bool {
        self.started && !self.cards.is_empty() && self.answered >= self.cards.len()
    }

    /// Reset back to the start screen, keeping the chosen count.
    fn reset(&mut self) {
        self.cards.clear();
        self.index = 0;
        self.revealed = false;
        self.started = false;
        self.loading = false;
        self.answered = 0;
        self.score = 0;
    }

    /// Grade the current card and advance. Returns the action to dispatch.
    fn grade(&mut self, quality: u8) -> Option<Action> {
        let id = self.cards.get(self.index).map(|c| c.id)?;
        if quality >= 3 {
            self.score += 1;
        }
        self.answered += 1;
        self.revealed = false;
        if self.index + 1 < self.cards.len() {
            self.index += 1;
        }
        Some(Action::GradeQuiz { id, quality })
    }

    // ---- per-screen key handling ----------------------------------------

    fn on_key_start(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Up | KeyCode::Right | KeyCode::Char('k') | KeyCode::Char('+') => {
                self.count = (self.count + 1).min(MAX_COUNT);
            }
            KeyCode::Down | KeyCode::Left | KeyCode::Char('j') | KeyCode::Char('-') => {
                self.count = self.count.saturating_sub(1).max(MIN_COUNT);
            }
            KeyCode::Enter => {
                self.loading = true;
                return Some(Action::StartQuiz { count: self.count });
            }
            _ => {}
        }
        None
    }

    fn on_key_question(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc => {
                self.reset();
                None
            }
            KeyCode::Char(' ') | KeyCode::Enter if !self.revealed => {
                self.revealed = true;
                None
            }
            KeyCode::Char(c @ '1'..='5') if self.revealed => self.grade(c as u8 - b'0'),
            _ => None,
        }
    }

    fn on_key_finished(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Enter | KeyCode::Char('r') => {
                self.reset();
            }
            _ => {}
        }
        None
    }
}

impl Pane for QuizState {
    fn on_key(&mut self, key: KeyEvent, _ctx: &Ctx) -> Option<Action> {
        if self.loading {
            // Manual dismiss (e.g. after a generation error surfaced as a toast).
            if key.code == KeyCode::Enter {
                self.reset();
            }
            None
        } else if self.finished() {
            self.on_key_finished(key)
        } else if self.in_question() {
            self.on_key_question(key)
        } else {
            self.on_key_start(key)
        }
    }

    fn apply(&mut self, msg: &Message) {
        match msg {
            Message::QuizGenerated(cards) => {
                self.cards = cards.clone();
                self.index = 0;
                self.revealed = false;
                self.answered = 0;
                self.score = 0;
                self.loading = false;
                self.started = true;
            }
            // Grading advances optimistically in `on_key`; this just confirms
            // the SM-2 write completed, so nothing more to fold in.
            Message::QuizGraded { id: _ } => {}
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, focused: bool) {
        let block = Block::bordered()
            .title(" Quiz ")
            .title_style(theme.title())
            .border_style(theme.border(focused));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.loading {
            self.render_loading(frame, inner, theme);
        } else if self.finished() {
            self.render_summary(frame, inner, theme);
        } else if self.in_question() {
            self.render_question(frame, inner, theme);
        } else {
            self.render_start(frame, inner, theme);
        }
    }

    fn wants_input(&self) -> bool {
        self.in_question()
    }
}

impl QuizState {
    fn render_start(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Generate a fresh quiz from your materials.",
                theme.dim(),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Questions:  ", theme.text()),
                Span::styled(format!("{}", self.count), theme.accent()),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  ↑/↓ (or +/-) adjust · Enter to start",
                theme.dim(),
            )),
            Line::from(Span::styled(
                "  Answers use active recall + SM-2 spaced repetition.",
                theme.dim(),
            )),
        ];
        frame.render_widget(Paragraph::new(lines), area);
    }

    fn render_loading(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  Generating a {}-question quiz…", self.count),
                theme.accent(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Retrieving your materials and prompting the model.",
                theme.dim(),
            )),
        ];
        frame.render_widget(Paragraph::new(lines), area);
    }

    fn render_question(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let Some(card) = self.cards.get(self.index) else {
            return;
        };

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(Span::styled(
            format!("  Question {}/{}", self.index + 1, self.cards.len()),
            theme.title(),
        )));
        lines.push(Line::from(""));

        for raw in card.front.split('\n') {
            lines.push(Line::from(Span::styled(format!("  {raw}"), theme.text())));
        }
        lines.push(Line::from(""));

        if self.revealed {
            lines.push(Line::from(Span::styled(
                "  Answer",
                theme.text().fg(theme.p.ok),
            )));
            for raw in card.back.split('\n') {
                lines.push(Line::from(Span::styled(format!("  {raw}"), theme.accent())));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Grade yourself:  1 again · 2 hard · 3 ok · 4 good · 5 easy",
                theme.dim(),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "  Space/Enter to reveal the answer · Esc to leave",
                theme.dim(),
            )));
        }

        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }

    fn render_summary(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let total = self.cards.len();
        let pct = if total > 0 {
            (self.score as f64 / total as f64 * 100.0).round() as u32
        } else {
            0
        };

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled("  Quiz complete", theme.title())),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Recalled:  ", theme.text()),
                Span::styled(
                    format!("{}/{} ({}%)", self.score, total, pct),
                    theme.accent(),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  Items were scheduled for spaced-repetition review.",
                theme.dim(),
            )),
            Line::from(Span::styled(
                "  Enter (or r) for another quiz · Esc for Home",
                theme.dim(),
            )),
        ];
        frame.render_widget(Paragraph::new(lines).alignment(Alignment::Left), area);
    }
}
