//! Thin event source wrapper (blueprint §1).
//!
//! Bundles the crossterm async [`EventStream`] with a steady tick timer so the
//! render loop can `select!` over both from one place. Kept intentionally small;
//! the loop reads `events.stream` and `events.tick` directly.

use std::time::Duration;

use crossterm::event::EventStream;
use tokio::time::{Interval, interval};

pub struct Events {
    /// Async stream of key/mouse/resize events.
    pub stream: EventStream,
    /// Steady tick for toasts and the spinner animation.
    pub tick: Interval,
}

impl Events {
    /// Create an event source ticking every `tick_ms` milliseconds.
    pub fn new(tick_ms: u64) -> Self {
        Self {
            stream: EventStream::new(),
            tick: interval(Duration::from_millis(tick_ms)),
        }
    }
}
