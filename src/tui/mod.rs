//! Full-screen terminal UI for The Librarian.
//!
//! This is the Phase 0 scaffold of Epic E3 (see `docs/design/TUI_ARCHITECTURE.md`).
//! It stands up the terminal, an async event/render loop, and a placeholder
//! three-region layout (library sidebar | main content | status bar). Screens,
//! the action/message plumbing, and the service layer arrive in later phases.

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers};
use futures_util::StreamExt;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use std::time::Duration;

/// Central application state. Grows into the full `App` model in Phase 1
/// (screen/focus/input-mode enums, per-screen sub-state, channels).
struct App {
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        Self { should_quit: false }
    }

    /// Translate a key press into a state change. Placeholder key handling until
    /// the keymap system (Phase 1) lands.
    fn on_key(&mut self, code: KeyCode, mods: KeyModifiers) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('c') if mods.contains(KeyModifiers::CONTROL) => self.should_quit = true,
            _ => {}
        }
    }
}

/// Launch the TUI. `ratatui::init` enables raw mode, enters the alternate
/// screen, and installs a panic hook that restores the terminal; `restore`
/// undoes it on the way out (even if the loop errors).
pub async fn run() -> Result<()> {
    let mut terminal = ratatui::init();
    let result = run_loop(&mut terminal).await;
    ratatui::restore();
    result
}

async fn run_loop(terminal: &mut DefaultTerminal) -> Result<()> {
    let mut app = App::new();
    let mut events = EventStream::new();
    // A steady tick drives animations/toasts later; harmless now.
    let mut tick = tokio::time::interval(Duration::from_millis(200));

    while !app.should_quit {
        terminal.draw(draw)?;

        tokio::select! {
            maybe_event = events.next() => {
                if let Some(Ok(Event::Key(key))) = maybe_event {
                    // Windows reports both press and release; act on press only.
                    if key.kind == KeyEventKind::Press {
                        app.on_key(key.code, key.modifiers);
                    }
                }
            }
            _ = tick.tick() => {}
        }
    }

    Ok(())
}

/// Render one frame: sidebar | main content, with a status bar underneath.
fn draw(frame: &mut Frame) {
    let accent = Style::default().fg(Color::Cyan);

    let [body, status] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());
    let [sidebar, main] =
        Layout::horizontal([Constraint::Length(28), Constraint::Min(0)]).areas(body);

    frame.render_widget(
        Paragraph::new("(no buckets yet)")
            .block(Block::bordered().title(" 📚 Library ").border_style(accent)),
        sidebar,
    );

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  The Librarian",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "  Your study companion",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from("  TUI scaffold — screens coming online (E3)."),
        ])
        .block(
            Block::bordered()
                .title(" The Librarian ")
                .border_style(accent),
        ),
        main,
    );

    let key_hint =
        |k: &'static str| Span::styled(k, Style::default().fg(Color::Black).bg(Color::Cyan));
    frame.render_widget(
        Line::from(vec![
            key_hint(" q "),
            Span::raw(" quit   "),
            key_hint(" Ctrl-C "),
            Span::raw(" quit "),
        ]),
        status,
    );
}
