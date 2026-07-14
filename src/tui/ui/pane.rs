//! The shared pane contract (blueprint §1, §2).
//!
//! Every Phase-2 screen (Search, Docs, Add, Study, Quiz, Review, Config) is a
//! self-contained module implementing [`Pane`]. `app.rs` owns one `…State` value
//! per pane and drives them **generically** through this trait, so a pane agent
//! edits only `ui/<pane>.rs` (+ `service/<pane>.rs`) and never a shared file.
//!
//! Contract:
//! - [`Pane::on_key`] handles a key while the pane is active, mutating local
//!   state and optionally returning an [`Action`] for the worker.
//! - [`Pane::apply`] folds a worker [`Message`] addressed to the pane into its
//!   state (called even when the pane is not the active screen).
//! - [`Pane::render`] paints the pane into the main region.
//! - [`Pane::wants_input`] reports whether the pane is currently capturing text
//!   (so `app.rs` suppresses global nav keys and lets the pane keep every key).

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use tokio::sync::mpsc::UnboundedSender;

use super::super::action::{Action, Message};
use super::super::theme::Theme;

/// Read-only globals + the action channel handed to a pane on each keypress so
/// it can build [`Action`]s without reaching into `App`.
#[derive(Clone)]
#[allow(dead_code)]
pub struct Ctx {
    pub current_bucket: Option<String>,
    pub has_api_key: bool,
    pub model: String,
    pub doc_count: i64,
    pub chunk_count: i64,
    /// For panes that need to dispatch follow-up actions outside `on_key`'s
    /// single return value (e.g. fire-and-forget refreshes).
    pub action_tx: UnboundedSender<Action>,
}

/// The uniform interface every Phase-2 pane implements.
pub trait Pane {
    /// Handle a key while this pane is the active screen. Return an [`Action`]
    /// to dispatch, or `None`.
    fn on_key(&mut self, key: KeyEvent, ctx: &Ctx) -> Option<Action>;

    /// Fold a worker [`Message`] addressed to this pane into local state. Panes
    /// should match only their own variants and ignore the rest.
    fn apply(&mut self, msg: &Message);

    /// Paint the pane into `area`. `focused` is true when the main region holds
    /// focus (vs. the sidebar).
    fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme, focused: bool);

    /// Whether the pane is currently capturing free text (default: no). When
    /// true, `app.rs` routes every key straight to [`Pane::on_key`] and skips
    /// global normal-mode keybindings.
    fn wants_input(&self) -> bool {
        false
    }
}

/// Themed "coming soon" placeholder shared by every not-yet-built pane.
pub fn coming_soon(frame: &mut Frame, area: Rect, theme: &Theme, title: &str, subtitle: &str) {
    let block = Block::bordered()
        .title(format!(" {title} "))
        .title_style(theme.title())
        .border_style(theme.border(false));
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {title} — coming soon"),
            theme.title(),
        )),
        Line::from(Span::styled(format!("  {subtitle}"), theme.dim())),
        Line::from(""),
        Line::from(Span::styled("  This pane arrives in Phase 2.", theme.dim())),
    ];
    frame.render_widget(Paragraph::new(lines).block(block), area);
}
