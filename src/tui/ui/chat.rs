//! Chat screen (blueprint §4): transcript + live stream buffer + input line.
//!
//! Text is pre-wrapped to the inner width so the transcript can auto-follow the
//! tail (and support scroll-back) with an exact line count. Markdown rendering
//! arrives in Phase 2; Phase 1 shows plain wrapped text.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Paragraph};

use super::super::app::{App, InputMode};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let [transcript, input] =
        Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).areas(area);

    render_transcript(frame, app, transcript);
    render_input(frame, app, input);
}

fn render_transcript(frame: &mut Frame, app: &App, area: Rect) {
    let title = match app.chat.conversation_id {
        Some(_) => " Chat ".to_string(),
        None => " Chat · new conversation ".to_string(),
    };
    let block = Block::bordered()
        .title(title)
        .title_style(app.theme.title())
        .border_style(app.theme.border(false));
    let inner = block.inner(area);
    let width = inner.width.max(1) as usize;
    let height = inner.height.max(1) as usize;

    let mut lines: Vec<Line> = Vec::new();

    if app.chat.history.iter().all(|m| m.role == "system") && !app.chat.streaming {
        empty_state(app, &mut lines);
    } else {
        for msg in &app.chat.history {
            match msg.role.as_str() {
                "user" => push_message(
                    &mut lines,
                    "You",
                    app.theme.user_msg(),
                    &msg.content,
                    width,
                    app,
                ),
                "assistant" => push_message(
                    &mut lines,
                    "Librarian",
                    app.theme.assistant_msg(),
                    &msg.content,
                    width,
                    app,
                ),
                _ => {}
            }
        }
        if app.chat.streaming {
            let body = format!("{}▌", app.chat.stream_buf);
            push_message(
                &mut lines,
                "Librarian",
                app.theme.assistant_msg(),
                &body,
                width,
                app,
            );
        }
    }

    // Auto-follow the tail, honoring scroll-back.
    let total = lines.len();
    let max_start = total.saturating_sub(height);
    let start = max_start.saturating_sub(app.chat.scroll as usize);
    let end = (start + height).min(total);
    let visible: Vec<Line> = lines[start..end].to_vec();

    frame.render_widget(Paragraph::new(Text::from(visible)).block(block), area);
}

fn empty_state(app: &App, lines: &mut Vec<Line>) {
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Ask the Librarian anything about your materials.",
        app.theme.dim(),
    )));
    lines.push(Line::from(Span::styled(
        "  Press i (or Enter) to compose, Enter to send.",
        app.theme.dim(),
    )));
    if !app.chat.conversations.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Recent conversations:",
            app.theme.text(),
        )));
        for conv in app.chat.conversations.iter().take(6) {
            lines.push(Line::from(Span::styled(
                format!("    • {}  ({})", conv.title, conv.updated_at),
                app.theme.dim(),
            )));
        }
    }
}

fn push_message(
    lines: &mut Vec<Line<'static>>,
    speaker: &str,
    speaker_style: Style,
    content: &str,
    width: usize,
    app: &App,
) {
    lines.push(Line::from(Span::styled(speaker.to_string(), speaker_style)));
    for raw in content.split('\n') {
        for wrapped in wrap(raw, width) {
            lines.push(Line::from(Span::styled(wrapped, app.theme.text())));
        }
    }
    lines.push(Line::from(""));
}

/// Simple greedy word-wrap to `width` columns (char count, not grapheme-aware).
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
            // Hard-break an over-long word.
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

fn render_input(frame: &mut Frame, app: &App, area: Rect) {
    let editing = app.input_mode == InputMode::Editing;
    let title = if editing {
        " Message (Enter to send) "
    } else {
        " Message (press i to compose) "
    };
    let block = Block::bordered()
        .title(title)
        .title_style(if editing {
            app.theme.title()
        } else {
            app.theme.dim()
        })
        .border_style(app.theme.border(editing));

    let mut content = app.chat.input.clone();
    if editing {
        content.push('▌');
    }
    let style = if editing {
        app.theme.text()
    } else {
        app.theme.dim().add_modifier(Modifier::DIM)
    };

    frame.render_widget(
        Paragraph::new(Span::styled(content, style)).block(block),
        area,
    );
}
