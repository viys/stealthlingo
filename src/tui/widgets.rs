//! Small building blocks shared by the screens.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Padding, Paragraph};
use ratatui::Frame;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::theme;
use crate::commands::lookup::pronunciation_line;
use crate::dictionary::Entry;
use crate::learning::spelling::mask_word;

/// A single-line text field.
#[derive(Debug, Clone, Default)]
pub struct LineInput {
    text: String,
    /// Cursor position in characters.
    cursor: usize,
}

impl LineInput {
    pub fn with_text(text: &str) -> Self {
        Self {
            text: text.to_string(),
            cursor: text.chars().count(),
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    fn byte_index(&self, chars: usize) -> usize {
        self.text
            .char_indices()
            .nth(chars)
            .map_or(self.text.len(), |(i, _)| i)
    }

    /// Applies an editing key. Returns false for keys the field does not use.
    pub fn handle(&mut self, key: KeyEvent) -> bool {
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return false;
        }
        let len = self.text.chars().count();
        match key.code {
            KeyCode::Char(c) => {
                let at = self.byte_index(self.cursor);
                self.text.insert(at, c);
                self.cursor += 1;
            }
            KeyCode::Backspace if self.cursor > 0 => {
                self.cursor -= 1;
                let at = self.byte_index(self.cursor);
                self.text.remove(at);
            }
            KeyCode::Delete if self.cursor < len => {
                let at = self.byte_index(self.cursor);
                self.text.remove(at);
            }
            KeyCode::Left => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Right => self.cursor = (self.cursor + 1).min(len),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = len,
            KeyCode::Backspace | KeyCode::Delete => {}
            _ => return false,
        }
        true
    }

    /// Draws `prompt` followed by the text, and places the terminal cursor.
    pub fn render(&self, frame: &mut Frame, area: Rect, prompt: Span<'static>) {
        let before = self.text[..self.byte_index(self.cursor)].width();
        let x = area.x as usize + prompt.width() + before;
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                prompt,
                Span::styled(self.text.clone(), theme::strong()),
            ])),
            area,
        );
        if x < (area.x + area.width) as usize {
            frame.set_cursor_position((x as u16, area.y));
        }
    }
}

pub fn is_ctrl(key: &KeyEvent, c: char) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char(c)
}

/// The middle `max_width` columns of `area`.
pub fn centered(area: Rect, max_width: u16) -> Rect {
    let [middle] = Layout::horizontal([Constraint::Max(max_width)])
        .flex(Flex::Center)
        .areas(area);
    middle
}

/// Padding for a bordered box holding `rows` of content: a blank row above and
/// below when `available` rows leave room for them, none otherwise.
pub fn box_padding(horizontal: u16, rows: u16, available: u16) -> Padding {
    let vertical = u16::from(rows.saturating_add(4) <= available);
    Padding::new(horizontal, horizontal, vertical, vertical)
}

/// Height of a bordered box with `padding` around `rows` of content, capped at
/// `available`.
pub fn box_height(rows: u16, padding: Padding, available: u16) -> u16 {
    rows.saturating_add(2 + padding.top + padding.bottom)
        .min(available)
}

/// First of `len` lines to show so that `selected` stays within `height` rows,
/// moving `offset` as little as possible.
pub fn scroll_to(offset: usize, selected: usize, height: usize, len: usize) -> usize {
    let height = height.max(1);
    let offset = if selected < offset {
        selected
    } else if selected >= offset + height {
        selected + 1 - height
    } else {
        offset
    };
    offset.min(len.saturating_sub(height))
}

/// Word-wraps by display width; words wider than a line (such as Chinese
/// text without spaces) are broken between characters.
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut used = 0;
    for word in text.split_whitespace() {
        let w = word.width();
        if used > 0 && used + 1 + w > width {
            lines.push(std::mem::take(&mut line));
            used = 0;
        }
        if w > width {
            for ch in word.chars() {
                let cw = ch.width().unwrap_or(0);
                if used > 0 && used + cw > width {
                    lines.push(std::mem::take(&mut line));
                    used = 0;
                }
                line.push(ch);
                used += cw;
            }
            continue;
        }
        if used > 0 {
            line.push(' ');
            used += 1;
        }
        line.push_str(word);
        used += w;
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

fn pad(text: &str, width: usize) -> String {
    let w = text.width();
    if text.is_empty() && width == 0 {
        String::new()
    } else if w >= width {
        format!("{text} ")
    } else {
        format!("{text}{}", " ".repeat(width - w))
    }
}

/// `text` wrapped into `width` columns with `label` in front of the first
/// line and continuation lines indented to line up with the text.
pub fn labelled(
    label: &str,
    label_style: Style,
    indent: usize,
    text: &str,
    text_style: Style,
    width: usize,
) -> Vec<Line<'static>> {
    wrap_text(text, width.saturating_sub(indent).max(10))
        .into_iter()
        .enumerate()
        .map(|(i, body)| {
            let lead = if i == 0 {
                Span::styled(pad(label, indent), label_style)
            } else {
                Span::raw(" ".repeat(indent))
            };
            Line::from(vec![lead, Span::styled(body, text_style)])
        })
        .collect()
}

/// `ephemeral   UK /ɪˈfɛm(ə)ɹəl/ · US /…/  ♪`
pub fn headline(entry: &Entry) -> Line<'static> {
    let mut spans = vec![Span::styled(entry.word.clone(), theme::heading())];
    let pronunciation = pronunciation_line(entry, false).or_else(|| entry.phonetic.clone());
    if let Some(p) = pronunciation {
        spans.push(Span::raw("   "));
        spans.push(Span::styled(p, theme::dim()));
    }
    if entry.has_audio() {
        let accents = entry.recording_accents().join(" ");
        spans.push(Span::styled(
            format!("  ♪ {accents}").trim_end().to_string(),
            theme::dim(),
        ));
    }
    Line::from(spans)
}

/// The first definition (and example) of up to two parts of speech, plus the
/// personal note. With `mask`, the word itself is blanked out.
pub fn meaning_lines(
    entry: &Entry,
    note: Option<&str>,
    mask: bool,
    width: usize,
) -> Vec<Line<'static>> {
    let hide = |text: &str| {
        if mask {
            mask_word(text, &entry.word)
        } else {
            text.to_string()
        }
    };
    let shown: Vec<_> = entry
        .meanings
        .iter()
        .filter_map(|m| {
            m.definitions
                .first()
                .map(|d| (m.part_of_speech.as_deref(), d))
        })
        .take(2)
        .collect();
    let indent = shown
        .iter()
        .map(|(pos, _)| pos.unwrap_or("meaning").width())
        .chain(note.map(|_| "note".len()))
        .max()
        .unwrap_or(0)
        .clamp(4, 14)
        + 2;
    let example_style = theme::dim().add_modifier(Modifier::ITALIC);

    let mut lines = Vec::new();
    for (pos, definition) in &shown {
        lines.extend(labelled(
            pos.unwrap_or("meaning"),
            theme::label(),
            indent,
            &hide(&definition.definition),
            Style::new(),
            width,
        ));
        if let Some(example) = &definition.example {
            lines.extend(labelled(
                "",
                Style::new(),
                indent,
                &format!("e.g. {}", hide(example)),
                example_style,
                width,
            ));
        }
    }
    if shown.is_empty() {
        lines.push(Line::styled("(no definition available)", theme::dim()));
    }
    if let Some(note) = note {
        lines.extend(labelled(
            "note",
            theme::label(),
            indent,
            note,
            theme::warn(),
            width,
        ));
    }
    lines
}

/// `━━━━━━──────` filled to `ratio`.
pub fn progress_bar(ratio: f64, width: usize) -> Vec<Span<'static>> {
    let filled = ((ratio.clamp(0.0, 1.0) * width as f64).round() as usize).min(width);
    vec![
        Span::styled("━".repeat(filled), theme::accent()),
        Span::styled("─".repeat(width - filled), theme::dim()),
    ]
}

pub fn clock(secs: u64) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_by_display_width() {
        assert_eq!(wrap_text("a bb ccc", 4), ["a bb", "ccc"]);
        assert_eq!(wrap_text("短暂的东西", 4), ["短暂", "的东", "西"]);
        assert_eq!(wrap_text("", 4), [""]);
    }

    #[test]
    fn scrolls_just_enough_to_show_the_selection() {
        assert_eq!(scroll_to(0, 2, 5, 8), 0);
        assert_eq!(scroll_to(0, 7, 5, 8), 3);
        assert_eq!(scroll_to(3, 4, 5, 8), 3);
        assert_eq!(scroll_to(3, 1, 5, 8), 1);
        // Once everything fits again, the list starts at the top.
        assert_eq!(scroll_to(3, 7, 8, 8), 0);
    }

    #[test]
    fn drops_box_padding_when_rows_are_short() {
        assert_eq!(box_padding(2, 4, 8), Padding::new(2, 2, 1, 1));
        assert_eq!(box_padding(2, 4, 7), Padding::new(2, 2, 0, 0));
        assert_eq!(box_height(4, box_padding(2, 4, 7), 7), 6);
        assert_eq!(box_height(10, box_padding(2, 10, 7), 7), 7);
    }

    #[test]
    fn edits_text_at_the_cursor() {
        let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
        let mut input = LineInput::with_text("cat");
        input.handle(key(KeyCode::Left));
        input.handle(key(KeyCode::Char('r')));
        assert_eq!(input.text(), "cart");
        input.handle(key(KeyCode::Backspace));
        input.handle(key(KeyCode::Home));
        input.handle(key(KeyCode::Delete));
        assert_eq!(input.text(), "at");
        assert!(!input.handle(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL)));
    }
}
