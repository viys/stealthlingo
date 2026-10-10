use anyhow::Result;
use chrono::Utc;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::widgets::{headline, labelled, LineInput};
use super::{theme, Fx, Target, Tone, View, Voice};
use crate::commands::lookup::{shown_meanings, Detail, BRIEF_DEFINITIONS_PER_MEANING};
use crate::commands::{fetch_word, Context, Freshness};
use crate::dictionary::clean_word;
use crate::storage::{AddOutcome, CachedWord};
use crate::time::format_local;

pub struct Lookup {
    input: LineInput,
    editing: bool,
    pending: Option<String>,
    result: Option<CachedWord>,
    /// Shown above the entry, e.g. when the cached copy is used offline.
    notice: Option<String>,
    detail: Detail,
    scroll: usize,
    view_height: usize,
    line_count: usize,
    back: Target,
    voice: Voice,
    /// Accent played first.
    accent: String,
}

impl Lookup {
    pub fn new(back: Target, accent: &str) -> Self {
        Self {
            input: LineInput::default(),
            editing: true,
            pending: None,
            result: None,
            notice: None,
            detail: Detail::Brief,
            scroll: 0,
            view_height: 0,
            line_count: 0,
            back,
            voice: Voice::default(),
            accent: accent.to_string(),
        }
    }

    pub fn with_entry(cached: CachedWord, back: Target, accent: &str) -> Self {
        Self {
            editing: false,
            result: Some(cached),
            ..Self::new(back, accent)
        }
    }

    fn toggle_saved(&mut self, ctx: &Context, fx: &mut Fx) -> Result<()> {
        let Some(cached) = &mut self.result else {
            return Ok(());
        };
        let word = cached.entry.word.clone();
        if cached.in_collection {
            ctx.db
                .archive_from_collection(&cached.headword, Utc::now())?;
            cached.in_collection = false;
            fx.flash(
                Tone::Info,
                format!("Removed \"{word}\". Press s to save it again with its progress."),
            );
        } else {
            let message = match ctx.db.add_to_collection(cached.id, None, Utc::now())? {
                AddOutcome::Restored => {
                    format!("Saved \"{word}\" again, with its previous progress.")
                }
                _ if !cached.entry.has_audio() => {
                    format!("Saved \"{word}\". It has no audio, so listening practice skips it.")
                }
                _ => format!("Saved \"{word}\" to your word list."),
            };
            cached.in_collection = true;
            fx.flash(Tone::Good, message);
        }
        Ok(())
    }

    fn entry_lines(&self, cached: &CachedWord, width: usize) -> Vec<Line<'static>> {
        let entry = &cached.entry;
        let full = self.detail == Detail::Full;
        let mut lines = vec![headline(entry)];
        lines.push(if cached.in_collection {
            Line::styled("✓ in your word list", theme::good())
        } else {
            Line::styled("not saved yet", theme::dim())
        });
        if let Some(notice) = &self.notice {
            lines.push(Line::styled(notice.clone(), theme::warn()));
        }
        if entry.meanings.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::styled("(no definitions available)", theme::dim()));
        }
        let example_style = theme::dim().add_modifier(Modifier::ITALIC);
        for meaning in shown_meanings(entry, self.detail) {
            lines.push(Line::raw(""));
            lines.push(Line::styled(
                meaning
                    .part_of_speech
                    .clone()
                    .unwrap_or_else(|| "(unknown part of speech)".to_string()),
                theme::label().add_modifier(Modifier::BOLD),
            ));
            let limit = if full {
                usize::MAX
            } else {
                BRIEF_DEFINITIONS_PER_MEANING
            };
            for (i, d) in meaning.definitions.iter().take(limit).enumerate() {
                lines.extend(labelled(
                    &format!("  {:>2}.", i + 1),
                    theme::dim(),
                    6,
                    &d.definition,
                    Style::new(),
                    width,
                ));
                if let Some(example) = d.example.as_ref().filter(|_| full) {
                    lines.extend(labelled(
                        "",
                        Style::new(),
                        6,
                        &format!("e.g. {example}"),
                        example_style,
                        width,
                    ));
                }
            }
            if full {
                for (label, words) in [
                    ("synonyms", &meaning.synonyms),
                    ("antonyms", &meaning.antonyms),
                ] {
                    if !words.is_empty() {
                        lines.extend(labelled(
                            &format!("  {label}"),
                            theme::label(),
                            12,
                            &words.join(", "),
                            theme::dim(),
                            width,
                        ));
                    }
                }
            }
        }
        if let Some(note) = &cached.note {
            lines.push(Line::raw(""));
            lines.extend(labelled(
                "note",
                theme::label(),
                6,
                note,
                theme::warn(),
                width,
            ));
        }
        if full {
            let source = match (entry.source_urls.first(), &entry.license) {
                (Some(url), Some(license)) => Some(format!("Source: {url} ({license})")),
                (Some(url), None) => Some(format!("Source: {url}")),
                (None, Some(license)) => Some(format!("License: {license}")),
                (None, None) => None,
            };
            if let Some(source) = source {
                lines.push(Line::raw(""));
                lines.push(Line::styled(source, theme::dim()));
            }
        }
        lines
    }

    fn scroll_by(&mut self, delta: isize) {
        let max = self.line_count.saturating_sub(self.view_height);
        self.scroll = self.scroll.saturating_add_signed(delta).min(max);
    }
}

impl View for Lookup {
    fn title(&self) -> String {
        "Look up".to_string()
    }

    fn hints(&self) -> Vec<(&'static str, &'static str)> {
        if self.editing {
            return vec![("Enter", "look up"), ("^C", "back")];
        }
        let saved = self.result.as_ref().is_some_and(|c| c.in_collection);
        let audio = self
            .result
            .as_ref()
            .map_or("audio", |c| self.voice.hint(&c.entry, &self.accent));
        vec![
            ("s", if saved { "remove" } else { "save" }),
            ("a", audio),
            (
                "Tab",
                if self.detail == Detail::Full {
                    "brief"
                } else {
                    "full"
                },
            ),
            ("↑↓", "scroll"),
            ("/", "new lookup"),
            ("q", "back"),
        ]
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let [input_row, _, body] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .areas(area);
        if self.editing {
            self.input.render(
                frame,
                input_row,
                Span::styled("Look up › ", theme::heading()),
            );
        } else {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("/", theme::heading()),
                    Span::styled(" new lookup", theme::dim()),
                ])),
                input_row,
            );
        }

        let lines = if let Some(word) = &self.pending {
            vec![Line::styled(
                format!("Looking up \"{word}\" on Wiktionary…"),
                theme::dim(),
            )]
        } else if let Some(cached) = &self.result {
            self.entry_lines(cached, body.width as usize)
        } else {
            vec![
                Line::styled("Type a word and press Enter.", theme::strong()),
                Line::styled(
                    "Definitions come from English Wiktionary; saved words work offline.",
                    theme::dim(),
                ),
            ]
        };
        self.view_height = body.height as usize;
        self.line_count = lines.len();
        self.scroll = self
            .scroll
            .min(self.line_count.saturating_sub(self.view_height));
        frame.render_widget(
            Paragraph::new(lines).scroll((self.scroll.min(u16::MAX as usize) as u16, 0)),
            body,
        );
    }

    fn handle_key(&mut self, ctx: &mut Context, key: KeyEvent, fx: &mut Fx) -> Result<()> {
        if self.editing {
            if key.code == KeyCode::Enter {
                let word = clean_word(self.input.text());
                if word.is_empty() {
                    self.input = LineInput::default();
                    fx.flash(Tone::Info, "Type a word to look up, then press Enter.");
                } else {
                    self.pending = Some(word);
                    self.editing = false;
                }
            } else {
                self.input.handle(key);
            }
            return Ok(());
        }
        let page = self.view_height.max(2) as isize - 1;
        match key.code {
            KeyCode::Char('/') | KeyCode::Char('i') => {
                self.input = LineInput::default();
                self.editing = true;
            }
            KeyCode::Char('s') => self.toggle_saved(ctx, fx)?,
            KeyCode::Char('a') => {
                if let Some(cached) = &self.result {
                    self.voice.play(&cached.entry, &self.accent, fx);
                }
            }
            KeyCode::Tab => {
                self.detail = match self.detail {
                    Detail::Brief => Detail::Full,
                    Detail::Full => Detail::Brief,
                };
                self.scroll = 0;
            }
            KeyCode::Up | KeyCode::Char('k') => self.scroll_by(-1),
            KeyCode::Down | KeyCode::Char('j') => self.scroll_by(1),
            KeyCode::PageUp => self.scroll_by(-page),
            KeyCode::PageDown | KeyCode::Char(' ') => self.scroll_by(page),
            KeyCode::Char('q') | KeyCode::Backspace => fx.go(self.back.clone()),
            _ => {}
        }
        Ok(())
    }

    fn interrupt(&mut self, _ctx: &mut Context, fx: &mut Fx) -> Result<bool> {
        if self.editing && self.result.is_some() {
            self.editing = false;
        } else {
            fx.go(self.back.clone());
        }
        Ok(true)
    }

    fn has_background_work(&self) -> bool {
        self.pending.is_some()
    }

    fn background_work(&mut self, ctx: &mut Context, _fx: &mut Fx) -> Result<()> {
        let Some(word) = self.pending.take() else {
            return Ok(());
        };
        match fetch_word(ctx, &word) {
            Ok((cached, freshness)) => {
                self.notice = match freshness {
                    Freshness::Fallback(err) => Some(format!(
                        "{err} Showing the copy cached on {}.",
                        format_local(cached.fetched_at)
                    )),
                    _ => None,
                };
                self.result = Some(cached);
                self.voice = Voice::default();
                self.scroll = 0;
                Ok(())
            }
            Err(err) => {
                self.editing = true;
                Err(err)
            }
        }
    }
}
