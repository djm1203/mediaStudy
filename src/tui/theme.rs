//! Theme + palette for the TUI (blueprint §5).
//!
//! Every widget draws through `app.theme` so a single toggle re-skins the whole
//! UI. Two modes ship in Phase 1: [`Theme::dark`] (default) and [`Theme::light`].

use ratatui::style::{Color, Modifier, Style};

use super::action::ToastLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

/// Semantic color slots. Widgets reference these, never raw colors.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: Color,
    pub surface: Color,
    pub fg: Color,
    pub fg_dim: Color,
    pub accent: Color,
    pub accent_alt: Color,
    pub ok: Color,
    pub warn: Color,
    pub err: Color,
    pub border: Color,
    pub border_focus: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub mode: ThemeMode,
    pub p: Palette,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            p: Palette {
                bg: Color::Rgb(16, 18, 24),
                surface: Color::Rgb(30, 33, 42),
                fg: Color::Rgb(220, 222, 232),
                fg_dim: Color::Rgb(128, 133, 150),
                accent: Color::Rgb(94, 170, 255),
                accent_alt: Color::Rgb(184, 142, 255),
                ok: Color::Rgb(120, 200, 130),
                warn: Color::Rgb(232, 190, 96),
                err: Color::Rgb(232, 110, 110),
                border: Color::Rgb(66, 72, 88),
                border_focus: Color::Rgb(94, 170, 255),
            },
        }
    }

    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            p: Palette {
                bg: Color::Rgb(250, 250, 252),
                surface: Color::Rgb(236, 238, 243),
                fg: Color::Rgb(30, 32, 40),
                fg_dim: Color::Rgb(110, 114, 126),
                accent: Color::Rgb(20, 110, 210),
                accent_alt: Color::Rgb(128, 66, 200),
                ok: Color::Rgb(28, 150, 62),
                warn: Color::Rgb(176, 128, 20),
                err: Color::Rgb(200, 52, 52),
                border: Color::Rgb(196, 200, 210),
                border_focus: Color::Rgb(20, 110, 210),
            },
        }
    }

    /// Toggle between dark and light.
    pub fn toggle(&mut self) {
        *self = match self.mode {
            ThemeMode::Dark => Self::light(),
            ThemeMode::Light => Self::dark(),
        };
    }

    /// Border style, brighter when the region is focused.
    pub fn border(&self, focused: bool) -> Style {
        Style::default().fg(if focused {
            self.p.border_focus
        } else {
            self.p.border
        })
    }

    /// Emphasized title style.
    pub fn title(&self) -> Style {
        Style::default()
            .fg(self.p.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for a user (question) message label.
    pub fn user_msg(&self) -> Style {
        Style::default()
            .fg(self.p.accent_alt)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for an assistant (answer) message label.
    pub fn assistant_msg(&self) -> Style {
        Style::default()
            .fg(self.p.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Primary foreground text.
    pub fn text(&self) -> Style {
        Style::default().fg(self.p.fg)
    }

    /// Dimmed/secondary text.
    pub fn dim(&self) -> Style {
        Style::default().fg(self.p.fg_dim)
    }

    /// Plain accent-colored text.
    pub fn accent(&self) -> Style {
        Style::default().fg(self.p.accent)
    }

    /// Style for a toast of the given level.
    pub fn toast(&self, level: ToastLevel) -> Style {
        let color = match level {
            ToastLevel::Info => self.p.accent,
            ToastLevel::Success => self.p.ok,
            ToastLevel::Warn => self.p.warn,
            ToastLevel::Error => self.p.err,
        };
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    }
}
