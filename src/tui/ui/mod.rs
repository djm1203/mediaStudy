//! Rendering entry point (blueprint §1, §3).
//!
//! `draw` lays out the persistent three-region frame — sidebar | main | status —
//! routes the main region to the active screen, and paints overlays on top.
//! Home + Chat are bespoke; the seven Phase-2 panes are driven generically
//! through the [`Pane`] trait (see [`pane`]).

mod add;
mod chat;
mod config;
mod docs;
mod home;
mod pane;
mod quiz;
mod review;
mod search;
mod sidebar;
mod statusbar;
mod study;

pub use add::AddState;
pub use config::ConfigState;
pub use docs::DocsState;
pub use pane::{Ctx, Pane};
pub use quiz::QuizState;
pub use review::ReviewState;
pub use search::SearchState;
pub use study::StudyState;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};

use super::app::{App, Focus, Overlay, Screen};
use super::keymap;

/// Render one frame.
pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Paint the themed background beneath everything.
    frame.render_widget(
        Block::default().style(Style::default().bg(app.theme.p.bg)),
        area,
    );

    let [body, status] = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);
    let [sidebar, main] =
        Layout::horizontal([Constraint::Length(28), Constraint::Min(0)]).areas(body);

    sidebar::render(frame, app, sidebar);

    // Home + Chat keep their bespoke renderers; every other screen renders
    // generically through its `Pane`.
    match app.screen {
        Screen::Home => home::render(frame, app, main),
        Screen::Chat => chat::render(frame, app, main),
        _ => {
            if let Some(p) = app.active_pane() {
                let focused = app.focus != Focus::Sidebar;
                p.render(frame, main, &app.theme, focused);
            }
        }
    }

    statusbar::render(frame, app, status);

    if app.overlay == Overlay::Help {
        render_help(frame, app, area);
    }
}

/// Centered help overlay generated from the keymap tables.
fn render_help(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(60, 80, area);
    frame.render_widget(Clear, popup);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled("  Global", app.theme.title())));
    push_hints(&mut lines, app, keymap::GLOBAL);
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  Navigation", app.theme.title())));
    push_hints(&mut lines, app, keymap::NAV);
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  Sidebar", app.theme.title())));
    push_hints(&mut lines, app, keymap::SIDEBAR);
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("  Chat", app.theme.title())));
    push_hints(&mut lines, app, keymap::CHAT);
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Press ? or Esc to close",
        app.theme.dim(),
    )));

    let block = Block::bordered()
        .title(" Help ")
        .border_style(app.theme.border(true));
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn push_hints(lines: &mut Vec<Line>, app: &App, hints: &[(&str, &str)]) {
    for (keys, desc) in hints {
        lines.push(Line::from(vec![
            Span::styled(format!("    {:<14}", keys), app.theme.accent()),
            Span::styled((*desc).to_string(), app.theme.text()),
        ]));
    }
}

/// Compute a centered rectangle taking `percent_x`/`percent_y` of `r`.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let [_, mid, _] = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .areas(r);
    let [_, center, _] = Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .areas(mid);
    center
}
