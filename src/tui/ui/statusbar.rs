//! Status bar (blueprint §4): context on the left, spinner + toast on the right.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};

use super::super::app::{App, InputMode};
use super::super::keymap;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let bucket = app
        .current_bucket
        .as_deref()
        .unwrap_or("(no bucket)")
        .to_string();

    let mut spans = vec![
        Span::styled(format!(" {} ", app.screen.title()), app.theme.title()),
        Span::styled("│ ", app.theme.dim()),
        Span::styled(bucket, app.theme.accent()),
        Span::styled(
            format!("  {} docs · {} chunks  ", app.doc_count, app.chunk_count),
            app.theme.dim(),
        ),
    ];

    let glyph = app.spinner_glyph();
    if !glyph.is_empty() {
        spans.push(Span::styled(
            format!("{} working… ", glyph),
            app.theme.accent(),
        ));
    }

    if let Some(toast) = &app.status {
        spans.push(Span::styled(
            format!("• {} ", toast.text),
            app.theme.toast(toast.level),
        ));
    } else {
        let editing = app.input_mode == InputMode::Editing;
        spans.push(Span::styled(keymap::status_hint(editing), app.theme.dim()));
    }

    frame.render_widget(Line::from(spans), area);
}
