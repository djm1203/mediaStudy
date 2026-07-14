//! Library sidebar (blueprint §4 — replaces `print_library_shelf`).
//!
//! Lists every bucket with the active one highlighted; when the sidebar is
//! focused, the selection cursor is shown for switching.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use super::super::app::{App, Focus};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Sidebar;
    let block = Block::bordered()
        .title(" Library ")
        .title_style(app.theme.title())
        .border_style(app.theme.border(focused));

    let mut lines: Vec<Line> = Vec::new();

    if app.buckets.is_empty() {
        lines.push(Line::from(Span::styled(
            "(no buckets yet)",
            app.theme.dim(),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Create one with:",
            app.theme.dim(),
        )));
        lines.push(Line::from(Span::styled(
            "librarian bucket create",
            app.theme.accent(),
        )));
    } else {
        for (i, name) in app.buckets.iter().enumerate() {
            let is_current = app.current_bucket.as_deref() == Some(name.as_str());
            let is_sel = focused && i == app.sidebar_sel;

            let marker = if is_current { "📖" } else { "📕" };
            let mut style = if is_current {
                app.theme.accent().add_modifier(Modifier::BOLD)
            } else {
                app.theme.text()
            };
            if is_sel {
                style = style.bg(app.theme.p.surface).add_modifier(Modifier::BOLD);
            }

            lines.push(Line::from(Span::styled(
                format!(" {} {}", marker, name),
                style,
            )));
        }
    }

    frame.render_widget(Paragraph::new(lines).block(block), area);
}
