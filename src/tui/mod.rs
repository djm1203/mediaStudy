//! Full-screen terminal UI for The Librarian.
//!
//! Phase 1 of Epic E3 (see `docs/design/TUI_ARCHITECTURE.md`) built the async
//! event/render loop, the `Action`/`Message` plumbing, the service + worker
//! layers, and a working Chat vertical slice (RAG context → streamed reply →
//! persisted turn). Home + Chat are live.
//!
//! Phase 2a adds the conflict-free pane scaffolding: the full `Action`/`Message`
//! surface for every screen, a uniform [`ui::Pane`] contract that `app`/`worker`
//! drive generically, per-pane state modules (`ui/<pane>.rs`) and per-pane
//! service stubs (`service/<pane>.rs`). The seven Phase-2 panes render themed
//! "coming soon" placeholders; a follow-on agent implements each in isolation.

mod action;
mod app;
mod event;
mod keymap;
mod service;
mod theme;
mod ui;
mod worker;

pub use app::Screen;

use anyhow::Result;
use futures_util::StreamExt;
use ratatui::DefaultTerminal;
use tokio::sync::mpsc::unbounded_channel;

use action::{Action, Message};
use app::App;
use event::Events;

/// Launch the TUI on the Home screen.
pub async fn run() -> Result<()> {
    run_on(Screen::Home).await
}

/// Launch the TUI on a specific screen. `ratatui::init` enables raw mode, enters
/// the alternate screen, and installs a panic hook that restores the terminal;
/// `restore` undoes it on the way out (even if the loop errors).
pub async fn run_on(screen: Screen) -> Result<()> {
    let mut terminal = ratatui::init();
    let result = run_loop(&mut terminal, screen).await;
    ratatui::restore();
    result
}

async fn run_loop(terminal: &mut DefaultTerminal, screen: Screen) -> Result<()> {
    let (action_tx, mut action_rx) = unbounded_channel::<Action>();
    let (msg_tx, mut msg_rx) = unbounded_channel::<Message>();

    let mut app = App::new(action_tx.clone(), screen);
    let mut events = Events::new(150);

    // Bootstrap: always load the library; if we open straight into Chat, also
    // fetch the recent conversation list.
    let _ = action_tx.send(Action::LoadLibrary);
    if screen == Screen::Chat {
        let _ = action_tx.send(Action::LoadConversations);
    }

    while !app.should_quit {
        terminal.draw(|f| ui::draw(f, &app))?;

        tokio::select! {
            maybe_event = events.stream.next() => {
                if let Some(Ok(ev)) = maybe_event
                    && let Some(action) = app.handle_event(ev)
                {
                    let _ = action_tx.send(action);
                }
            }
            Some(msg) = msg_rx.recv() => {
                app.update(msg);
            }
            Some(action) = action_rx.recv() => {
                app.inflight += 1;
                worker::dispatch(action, msg_tx.clone());
            }
            _ = events.tick.tick() => {
                app.on_tick();
            }
        }
    }

    Ok(())
}
