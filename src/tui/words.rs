use anyhow::Result;
use chrono::Utc;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::Frame;

use super::widgets::{headline, labelled, LineInput};
use super::{theme, Fx, Target, Tone, View, Voice};
use crate::commands::Context;
use crate::dictionary::normalize_headword;
use crate::learning::Status;
use crate::storage::{CachedWord, WordSummary};
use crate::time::describe_due;

enum Mode {
    Browse,
    Filter,
    Note(LineInput),
    ConfirmRemove,
}

pub struct Words {
    all: Vec<WordSummary>,
    /// Indices into `all` that match the filter.
    shown: Vec<usize>,
    filter: LineInput,
    mode: Mode,
    table: TableState,
    selected: Option<CachedWord>,
    page: usize,
    voice: Voice,
    /// Accent played first.
    accent: String,
}

fn status_style(status: Status) -> Style {
    match status {
        Status::New => theme::accent(),
        Status::Learning => theme::warn(),
        Status::Review => Style::new(),
        Status::Mastered => theme::good(),
    }
}

impl Words {
    pub fn load(ctx: &Context) -> Result<Self> {
        let mut words = Self {
            all: Vec::new(),
            shown: Vec::new(),
            filter: LineInput::default(),
            mode: Mode::Browse,
            table: TableState::default(),
            selected: None,
            page: 10,
            voice: Voice::default(),
            accent: ctx.config.accent.clone(),
        };
        words.reload(ctx)?;
        Ok(words)
    }

    /// Re-reads the list, keeping the selection on the same word if possible.
    fn reload(&mut self, ctx: &Context) -> Result<()> {
        let keep = self.current().map(|w| w.word_id);
        self.all = ctx.db.list_words(None)?;
        self.apply_filter();
        if let Some(pos) =
            keep.and_then(|id| self.shown.iter().position(|&i| self.all[i].word_id == id))
        {
            self.table.select(Some(pos));
        }
        self.load_selected(ctx)
    }

    fn apply_filter(&mut self) {
        let query = self.filter.text().trim().to_lowercase();
        self.shown = self
            .all
            .iter()
            .enumerate()
            .filter(|(_, w)| {
                query.is_empty()
                    || w.display_word.to_lowercase().contains(&query)
                    || w.note
                        .as_deref()
                        .is_some_and(|n| n.to_lowercase().contains(&query))
            })
            .map(|(i, _)| i)
            .collect();
        let selected = match self.shown.len() {
            0 => None,
            n => Some(self.table.selected().unwrap_or(0).min(n - 1)),
        };
        self.table.select(selected);
    }

    fn current(&self) -> Option<&WordSummary> {
        self.table
            .selected()
            .and_then(|i| self.shown.get(i))
            .map(|&i| &self.all[i])
    }

    fn load_selected(&mut self, ctx: &Context) -> Result<()> {
        self.voice = Voice::default();
        self.selected = match self.current() {
            Some(word) => ctx
                .db
                .find_cached(&normalize_headword(&word.display_word))?,
            None => None,
        };
        Ok(())
    }

    fn move_by(&mut self, ctx: &Context, delta: isize) -> Result<()> {
        if self.shown.is_empty() {
            return Ok(());
        }
        let last = self.shown.len() - 1;
        let at = self.table.selected().unwrap_or(0);
        self.table
            .select(Some(at.saturating_add_signed(delta).min(last)));
        self.load_selected(ctx)
    }

    fn browse_key(&mut self, ctx: &mut Context, key: KeyEvent, fx: &mut Fx) -> Result<()> {
        let page = self.page as isize;
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_by(ctx, -1)?,
            KeyCode::Down | KeyCode::Char('j') => self.move_by(ctx, 1)?,
            KeyCode::PageUp => self.move_by(ctx, -page)?,
            KeyCode::PageDown => self.move_by(ctx, page)?,
            KeyCode::Home | KeyCode::Char('g') => self.move_by(ctx, isize::MIN)?,
            KeyCode::End | KeyCode::Char('G') => self.move_by(ctx, isize::MAX)?,
            KeyCode::Char('/') => self.mode = Mode::Filter,
            KeyCode::Enter => {
                if let Some(cached) = &self.selected {
                    fx.go(Target::Entry(Box::new(cached.clone())));
                }
            }
            KeyCode::Char('a') => {
                if let Some(cached) = &self.selected {
                    self.voice.play(&cached.entry, &self.accent, fx);
                }
            }
            KeyCode::Char('n') => {
                if let Some(word) = self.current() {
                    self.mode =
                        Mode::Note(LineInput::with_text(word.note.as_deref().unwrap_or("")));
                }
            }
            KeyCode::Char('d') => {
                if self.current().is_some() {
                    self.mode = Mode::ConfirmRemove;
                }
            }
            KeyCode::Char('q') | KeyCode::Backspace => fx.go(Target::Home),
            _ => {}
        }
        Ok(())
    }

    fn detail_lines(&self, width: usize) -> Vec<Line<'static>> {
        let (Some(word), Some(cached)) = (self.current(), &self.selected) else {
            return Vec::new();
        };
        let mut lines = vec![headline(&cached.entry)];
        lines.extend(labelled(
            "",
            Style::new(),
            0,
            &cached.entry.short_summary(),
            Style::new(),
            width,
        ));
        match &self.mode {
            Mode::Note(_) => {}
            Mode::ConfirmRemove => lines.push(Line::from(vec![
                Span::styled(format!("Remove \"{}\"? ", word.display_word), theme::bad()),
                Span::styled("y", theme::heading()),
                Span::styled(" yes   any other key: no", theme::dim()),
            ])),
            _ => {
                if let Some(note) = &word.note {
                    lines.extend(labelled(
                        "note",
                        theme::label(),
                        6,
                        note,
                        theme::warn(),
                        width,
                    ));
                }
            }
        }
        lines
    }
}

impl View for Words {
    fn title(&self) -> String {
        format!("Word list ({})", self.all.len())
    }

    fn hints(&self) -> Vec<(&'static str, &'static str)> {
        match self.mode {
            Mode::Browse => vec![
                ("↑↓", "move"),
                ("Enter", "details"),
                ("/", "filter"),
                (
                    "a",
                    self.selected
                        .as_ref()
                        .map_or("audio", |c| self.voice.hint(&c.entry, &self.accent)),
                ),
                ("n", "note"),
                ("d", "remove"),
                ("q", "back"),
            ],
            Mode::Filter => vec![("Enter", "done"), ("↑↓", "move"), ("^C", "clear")],
            Mode::Note(_) => vec![("Enter", "save note"), ("^C", "cancel")],
            Mode::ConfirmRemove => vec![("y", "remove"), ("any key", "cancel")],
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let filtering = matches!(self.mode, Mode::Filter) || !self.filter.text().is_empty();
        let [filter_row, table_area, detail_area] = Layout::vertical([
            Constraint::Length(if filtering { 2 } else { 0 }),
            Constraint::Min(3),
            Constraint::Length(7),
        ])
        .areas(area);
        self.page = table_area.height.saturating_sub(2).max(1) as usize;

        if filtering {
            let prompt = Span::styled("Filter › ", theme::heading());
            let row = Rect {
                height: 1,
                ..filter_row
            };
            if matches!(self.mode, Mode::Filter) {
                self.filter.render(frame, row, prompt);
            } else {
                frame.render_widget(
                    Paragraph::new(Line::from(vec![
                        prompt,
                        Span::raw(self.filter.text().to_string()),
                    ])),
                    row,
                );
            }
        }

        if self.all.is_empty() {
            frame.render_widget(
                Paragraph::new(vec![
                    Line::styled("No saved words yet.", theme::strong()),
                    Line::styled(
                        "Look one up from the home screen (/) and press s to save it.",
                        theme::dim(),
                    ),
                ]),
                table_area,
            );
            return;
        }

        let now = Utc::now();
        let word_width = self
            .all
            .iter()
            .map(|w| w.display_word.chars().count())
            .max()
            .unwrap_or(4)
            .clamp(4, 24) as u16;
        let rows: Vec<Row> = self
            .shown
            .iter()
            .map(|&i| {
                let w = &self.all[i];
                let due = if w.status == Status::New {
                    "new".to_string()
                } else {
                    describe_due(w.due_at, now)
                };
                let detail = match &w.note {
                    Some(note) => Line::from(vec![
                        Span::styled(note.clone(), theme::warn()),
                        Span::styled(format!("  {}", w.summary), theme::dim()),
                    ]),
                    None => Line::styled(w.summary.clone(), theme::dim()),
                };
                Row::new(vec![
                    Cell::from(Span::styled(w.display_word.clone(), theme::strong())),
                    Cell::from(Span::styled(w.status.as_str(), status_style(w.status))),
                    Cell::from(Span::styled(due, theme::dim())),
                    Cell::from(detail),
                ])
            })
            .collect();
        let table = Table::new(
            rows,
            [
                Constraint::Length(word_width),
                Constraint::Length(9),
                Constraint::Length(7),
                Constraint::Fill(1),
            ],
        )
        .header(
            Row::new(["word", "status", "due", "note / definition"])
                .style(theme::dim().add_modifier(Modifier::BOLD)),
        )
        .column_spacing(2)
        .row_highlight_style(theme::selected())
        .highlight_symbol("› ");
        frame.render_stateful_widget(table, table_area, &mut self.table);

        let block = Block::new()
            .borders(Borders::TOP)
            .border_style(theme::dim());
        let inner = block.inner(detail_area);
        frame.render_widget(block, detail_area);
        let width = inner.width as usize;
        let lines = self.detail_lines(width);
        let used = lines.len() as u16;
        frame.render_widget(Paragraph::new(lines), inner);
        if let Mode::Note(input) = &self.mode {
            if inner.height > used {
                let row = Rect {
                    y: inner.y + used,
                    height: 1,
                    ..inner
                };
                input.render(frame, row, Span::styled("note › ", theme::heading()));
            }
        }
        if self.shown.is_empty() {
            frame.render_widget(
                Paragraph::new(Line::styled(
                    "No saved words match the filter.",
                    theme::dim(),
                )),
                inner,
            );
        }
    }

    fn handle_key(&mut self, ctx: &mut Context, key: KeyEvent, fx: &mut Fx) -> Result<()> {
        match &mut self.mode {
            Mode::Browse => self.browse_key(ctx, key, fx)?,
            Mode::Filter => match key.code {
                KeyCode::Enter => self.mode = Mode::Browse,
                KeyCode::Up => self.move_by(ctx, -1)?,
                KeyCode::Down => self.move_by(ctx, 1)?,
                _ => {
                    if self.filter.handle(key) {
                        self.apply_filter();
                        self.load_selected(ctx)?;
                    }
                }
            },
            Mode::Note(input) => {
                if key.code == KeyCode::Enter {
                    let note = input.text().trim().to_string();
                    self.mode = Mode::Browse;
                    if let Some(word) = self.current() {
                        let (id, name) = (word.word_id, word.display_word.clone());
                        if note.is_empty() {
                            fx.flash(Tone::Info, "Notes cannot be emptied; the old note is kept.");
                        } else {
                            ctx.db.add_to_collection(id, Some(&note), Utc::now())?;
                            fx.flash(Tone::Good, format!("Saved the note for \"{name}\"."));
                        }
                    }
                    self.reload(ctx)?;
                } else {
                    input.handle(key);
                }
            }
            Mode::ConfirmRemove => {
                self.mode = Mode::Browse;
                if key.code == KeyCode::Char('y') {
                    if let Some(word) = self.current() {
                        let name = word.display_word.clone();
                        ctx.db
                            .archive_from_collection(&normalize_headword(&name), Utc::now())?;
                        fx.flash(
                            Tone::Info,
                            format!("Removed \"{name}\". Saving it again restores its progress."),
                        );
                    }
                    self.reload(ctx)?;
                }
            }
        }
        Ok(())
    }

    fn interrupt(&mut self, ctx: &mut Context, _fx: &mut Fx) -> Result<bool> {
        match self.mode {
            Mode::Browse => return Ok(false),
            Mode::Filter => {
                self.filter = LineInput::default();
                self.apply_filter();
                self.load_selected(ctx)?;
            }
            Mode::Note(_) | Mode::ConfirmRemove => {}
        }
        self.mode = Mode::Browse;
        Ok(true)
    }
}
