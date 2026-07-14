//! Rendering entry point (blueprint §1, §3).
//!
//! `draw` lays out the persistent three-region frame — sidebar | main | status —
//! routes the main region to the active screen, and paints overlays on top.

mod chat;
mod home;
mod sidebar;
mod statusbar;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};

use super::app::{App, Overlay, Screen};
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

    match app.screen {
        Screen::Home => home::render(frame, app, main),
        Screen::Chat => chat::render(frame, app, main),
        other => placeholder(frame, app, main, other),
    }

    statusbar::render(frame, app, status);

    if app.overlay == Overlay::Help {
        render_help(frame, app, area);
    }
}

/// "Coming soon" placeholder for screens not yet built (Phase 2).
fn placeholder(frame: &mut Frame, app: &App, area: Rect, screen: Screen) {
    let block = Block::bordered()
        .title(format!(" {} ", screen.title()))
        .border_style(app.theme.border(false));
    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {} — coming soon", screen.title()),
            app.theme.title(),
        )),
        Line::from(Span::styled(
            "  This screen arrives in Phase 2.",
            app.theme.dim(),
        )),
    ];
    frame.render_widget(Paragraph::new(text).block(block), area);
}

/// Centered help overlay generated from the keymap tables.
fn render_help(frame: &mut Frame, app: &App, area: Rect) {
    let popup = centered_rect(60, 70, area);
    frame.render_widget(Clear, popup);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled("  Global", app.theme.title())));
    push_hints(&mut lines, app, keymap::GLOBAL);
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
