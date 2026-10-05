//! Colors and styles shared by every TUI view.
//!
//! Text uses the terminal's default foreground so the UI stays readable on light and dark
//! themes; color is reserved for focus, state, and HTTP semantics.

use crate::config::types::GemonMethodType;
use ratatui::style::{Color, Modifier, Style};

pub const ACCENT: Color = Color::Cyan;
pub const MUTED: Color = Color::DarkGray;
pub const SUCCESS: Color = Color::Green;
pub const WARNING: Color = Color::Yellow;
pub const DANGER: Color = Color::Red;

pub fn method_color(method: GemonMethodType) -> Color {
    match method {
        GemonMethodType::Get => Color::Green,
        GemonMethodType::Post => Color::Yellow,
        GemonMethodType::Put => Color::Blue,
        GemonMethodType::Patch => Color::Magenta,
        GemonMethodType::Delete => Color::Red,
    }
}

pub fn method(method: GemonMethodType) -> Style {
    Style::new()
        .fg(method_color(method))
        .add_modifier(Modifier::BOLD)
}

pub fn status(code: u16) -> Style {
    let color = match code {
        200..=299 => SUCCESS,
        300..=399 => ACCENT,
        400..=499 => WARNING,
        500..=599 => DANGER,
        _ => MUTED,
    };
    Style::new().fg(color).add_modifier(Modifier::BOLD)
}

pub fn border(focused: bool) -> Style {
    Style::new().fg(if focused { ACCENT } else { MUTED })
}

pub fn title(focused: bool) -> Style {
    if focused {
        Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::new().add_modifier(Modifier::BOLD)
    }
}

pub fn text() -> Style {
    Style::new()
}

pub fn muted() -> Style {
    Style::new().fg(MUTED)
}

pub fn placeholder() -> Style {
    Style::new().fg(MUTED).add_modifier(Modifier::ITALIC)
}

pub fn key() -> Style {
    Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)
}

pub fn label() -> Style {
    Style::new().fg(MUTED).add_modifier(Modifier::BOLD)
}

pub fn heading() -> Style {
    Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)
}

pub fn selected(focused: bool) -> Style {
    if focused {
        Style::new()
            .fg(Color::Black)
            .bg(ACCENT)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)
    }
}

pub fn active_tab(focused: bool) -> Style {
    if focused {
        Style::new()
            .fg(Color::Black)
            .bg(ACCENT)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::new()
            .fg(ACCENT)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    }
}

pub fn success() -> Style {
    Style::new().fg(SUCCESS)
}

pub fn warning() -> Style {
    Style::new().fg(WARNING)
}

pub fn danger() -> Style {
    Style::new().fg(DANGER).add_modifier(Modifier::BOLD)
}

pub fn json_key() -> Style {
    Style::new().fg(Color::Cyan)
}

pub fn json_string() -> Style {
    Style::new().fg(Color::Green)
}

pub fn json_number() -> Style {
    Style::new().fg(Color::Yellow)
}

pub fn json_literal() -> Style {
    Style::new().fg(Color::Magenta)
}

pub fn json_punctuation() -> Style {
    Style::new().fg(MUTED)
}

pub fn search_match() -> Style {
    Style::new()
        .fg(Color::Black)
        .bg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

pub fn search_current() -> Style {
    Style::new()
        .fg(Color::Black)
        .bg(Color::LightRed)
        .add_modifier(Modifier::BOLD)
}
