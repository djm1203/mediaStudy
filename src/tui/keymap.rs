//! Keymap metadata (blueprint §5).
//!
//! Phase 2 keeps this as static `(keys, description)` tables that feed both the
//! status-bar hint and the help overlay. Later phases add per-screen chord
//! resolution here.

/// Global keybindings, always active.
pub const GLOBAL: &[(&str, &str)] = &[
    ("q / Ctrl-C", "Quit"),
    ("?", "Toggle help"),
    ("Tab", "Cycle focus"),
    ("Ctrl-T", "Toggle theme"),
    ("Esc", "Back to Home"),
];

/// Screen navigation (number keys), reachable from any non-editing screen.
pub const NAV: &[(&str, &str)] = &[
    ("1", "Home"),
    ("2", "Chat"),
    ("3", "Search"),
    ("4", "Docs"),
    ("5", "Add"),
    ("6", "Study"),
    ("7", "Quiz"),
    ("8", "Review"),
    ("9", "Config"),
];

/// Sidebar-focused keybindings.
pub const SIDEBAR: &[(&str, &str)] = &[("j / k", "Move selection"), ("Enter", "Switch bucket")];

/// Chat-screen keybindings.
pub const CHAT: &[(&str, &str)] = &[
    ("i / Enter", "Compose message"),
    ("Enter", "Send (while composing)"),
    ("Esc", "Stop composing"),
    ("n", "New conversation"),
    ("j / k", "Scroll transcript"),
];

/// One-line hint shown in the status bar for the given context.
pub fn status_hint(editing: bool) -> &'static str {
    if editing {
        "Enter send · Esc cancel · Ctrl-C quit"
    } else {
        "? help · 1-9 screens · Tab focus · q quit"
    }
}
