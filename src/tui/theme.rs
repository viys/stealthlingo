//! Colors and text styles. Colors are dropped when `NO_COLOR` is set.

use std::sync::OnceLock;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

fn colors() -> bool {
    static COLORS: OnceLock<bool> = OnceLock::new();
    *COLORS.get_or_init(|| std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty()))
}

fn fg(color: Color) -> Style {
    if colors() {
        Style::new().fg(color)
    } else {
        Style::new()
    }
}

pub fn accent() -> Style {
    fg(Color::Cyan)
}

pub fn good() -> Style {
    fg(Color::Green)
}

pub fn bad() -> Style {
    fg(Color::Red)
}

pub fn warn() -> Style {
    fg(Color::Yellow)
}

pub fn dim() -> Style {
    if colors() {
        Style::new().fg(Color::DarkGray)
    } else {
        Style::new().add_modifier(Modifier::DIM)
    }
}

pub fn strong() -> Style {
    Style::new().add_modifier(Modifier::BOLD)
}

pub fn heading() -> Style {
    accent().add_modifier(Modifier::BOLD)
}

pub fn label() -> Style {
    accent().add_modifier(Modifier::ITALIC)
}

pub fn selected() -> Style {
    Style::new().add_modifier(Modifier::REVERSED)
}

/// Key hints such as `Space reveal  a audio`.
pub fn hints(pairs: &[(&'static str, &'static str)]) -> Line<'static> {
    let mut spans = Vec::with_capacity(pairs.len() * 3);
    for (i, (key, action)) in pairs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("   "));
        }
        spans.push(Span::styled(*key, heading()));
        spans.push(Span::styled(format!(" {action}"), dim()));
    }
    Line::from(spans)
}
