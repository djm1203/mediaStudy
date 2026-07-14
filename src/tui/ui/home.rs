//! Home dashboard (blueprint §4 — replaces `print_dashboard` + main menu).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use super::super::app::App;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::bordered()
        .title(" The Librarian ")
        .title_style(app.theme.title())
        .border_style(app.theme.border(false));

    let bucket = app
        .current_bucket
        .as_deref()
        .unwrap_or("(no bucket selected)")
        .to_string();

    let key_status = if app.has_api_key {
        Span::styled("Ready", app.theme.text().fg(app.theme.p.ok))
    } else {
        Span::styled("Not configured", app.theme.text().fg(app.theme.p.err))
    };

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Your personal study companion",
            app.theme.dim(),
        )),
        Line::from(""),
        row(
            app,
            "📖 Current Book:",
            Span::styled(bucket, app.theme.accent()),
        ),
        row(
            app,
            "📄 Contents:",
            Span::styled(
                format!("{} documents · {} chunks", app.doc_count, app.chunk_count),
                app.theme.text(),
            ),
        ),
        row(app, "🔑 API Key:", key_status),
        row(
            app,
            "🤖 Model:",
            Span::styled(app.model.clone(), app.theme.text().fg(app.theme.p.warn)),
        ),
        Line::from(""),
        Line::from(Span::styled(
            "  2 Chat · 3 Search · 4 Docs · 5 Add · 6 Study · 7 Quiz · 8 Review · 9 Config",
            app.theme.dim(),
        )),
        Line::from(Span::styled(
            "  Tab to focus the library · ? for help",
            app.theme.dim(),
        )),
    ];

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn row<'a>(app: &App, label: &'a str, value: Span<'a>) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("  {:<16} ", label), app.theme.text()),
        value,
    ])
}
