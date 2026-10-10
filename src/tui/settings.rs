use anyhow::Result;
use chrono::Utc;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use super::widgets::{self, LineInput};
use super::{theme, Fx, Target, Tone, View};
use crate::commands::Context;
use crate::config::{Config, Kind, Setting, SETTINGS};
use crate::time::local_day_start;

/// Width of the setting names column.
const LABEL_WIDTH: usize = 22;
/// Columns before the value: the selection marker and the names.
const VALUE_X: u16 = 3 + LABEL_WIDTH as u16;
/// The allowed range is shown from this body width up.
const RANGE_MIN_WIDTH: u16 = 56;

enum Mode {
    Browse,
    /// Typing a number for the selected setting.
    Typing(LineInput),
    ConfirmResetAll,
}

pub struct Settings {
    /// What is in `config.json`, kept in step with the context after every
    /// change.
    config: Config,
    selected: usize,
    /// First line of the list shown when it does not fit.
    offset: usize,
    mode: Mode,
    /// Today's numbers, for telling whether the daily goal can be reached.
    words_today: i64,
    due: i64,
    new_ready: i64,
    new_started_today: i64,
    config_file: String,
    data_dir: String,
}

/// One line of the list: a group heading or a setting (index into `SETTINGS`).
enum Row {
    Group(&'static str),
    Setting(usize),
}

fn rows() -> Vec<Row> {
    let mut rows = Vec::new();
    let mut group = None;
    for (i, setting) in SETTINGS.iter().enumerate() {
        if group != Some(setting.group) {
            group = Some(setting.group);
            rows.push(Row::Group(setting.group.label()));
        }
        rows.push(Row::Setting(i));
    }
    rows
}

impl Settings {
    pub fn load(ctx: &Context) -> Result<Self> {
        let now = Utc::now();
        let stats = ctx.db.stats(now, local_day_start(now))?;
        Ok(Self {
            config: ctx.config.clone(),
            selected: 0,
            offset: 0,
            mode: Mode::Browse,
            words_today: stats.words_today,
            due: stats.due_now,
            new_ready: ctx.db.count_new_ready()?,
            new_started_today: stats.new_words_today,
            config_file: ctx.paths.config_file().display().to_string(),
            data_dir: ctx.paths.data_dir.display().to_string(),
        })
    }

    fn setting(&self) -> &'static Setting {
        &SETTINGS[self.selected]
    }

    /// Most different words that can be practised today with the current
    /// word list and `daily_new_limit`.
    fn reachable_today(&self) -> i64 {
        let allowance = (i64::from(self.config.daily_new_limit) - self.new_started_today).max(0);
        self.words_today + self.due + self.new_ready.min(allowance)
    }

    /// Applies `change` and saves the file right away. A failed save puts
    /// the old values back.
    fn change(&mut self, ctx: &mut Context, fx: &mut Fx, change: impl FnOnce(&mut Config)) {
        let before = ctx.config.clone();
        change(&mut ctx.config);
        if ctx.config == before {
            return;
        }
        match ctx.save_config() {
            Ok(()) => fx.flash(Tone::Good, "Saved"),
            Err(err) => {
                ctx.config = before;
                fx.flash(Tone::Bad, format!("Not saved: {err:#}"));
            }
        }
        self.config = ctx.config.clone();
    }

    fn step(&mut self, ctx: &mut Context, fx: &mut Fx, delta: i64) {
        let setting = self.setting();
        self.change(ctx, fx, |config| {
            setting.step(config, delta);
        });
    }

    fn select(&mut self, delta: isize) {
        self.selected = self
            .selected
            .saturating_add_signed(delta)
            .min(SETTINGS.len() - 1);
    }

    fn browse_key(&mut self, ctx: &mut Context, key: KeyEvent, fx: &mut Fx) {
        let setting = self.setting();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.select(-1),
            KeyCode::Down | KeyCode::Char('j') => self.select(1),
            KeyCode::Left | KeyCode::Char('-') => self.step(ctx, fx, -1),
            KeyCode::Right | KeyCode::Char('+') | KeyCode::Char('=') => self.step(ctx, fx, 1),
            KeyCode::PageUp => self.step(ctx, fx, 10),
            KeyCode::PageDown => self.step(ctx, fx, -10),
            KeyCode::Enter => match setting.kind {
                Kind::Number { .. } => self.mode = Mode::Typing(LineInput::default()),
                Kind::Accent => self.step(ctx, fx, 1),
            },
            KeyCode::Char('r') => self.change(ctx, fx, |config| setting.reset(config)),
            KeyCode::Char('R') => self.mode = Mode::ConfirmResetAll,
            KeyCode::Char('q') | KeyCode::Backspace => fx.go(Target::Home),
            _ => {}
        }
    }

    fn typing_key(&mut self, ctx: &mut Context, key: KeyEvent, fx: &mut Fx) {
        let Mode::Typing(input) = &mut self.mode else {
            return;
        };
        match key.code {
            KeyCode::Enter => {
                let text = input.text().to_string();
                if text.is_empty() {
                    self.mode = Mode::Browse;
                    return;
                }
                let setting = self.setting();
                let mut candidate = ctx.config.clone();
                match setting.set(&mut candidate, &text) {
                    Ok(()) => {
                        self.mode = Mode::Browse;
                        self.change(ctx, fx, |config| *config = candidate);
                    }
                    Err(err) => fx.flash(Tone::Bad, format!("{err:#}. Not saved.")),
                }
            }
            KeyCode::Up | KeyCode::Down => {
                self.mode = Mode::Browse;
                self.select(if key.code == KeyCode::Up { -1 } else { 1 });
            }
            KeyCode::Char(c) if !c.is_ascii_digit() || input.text().len() >= 6 => {}
            _ => {
                input.handle(key);
            }
        }
    }

    /// The list lines and the line of the selected setting.
    fn list_lines(&self, width: u16) -> (Vec<Line<'static>>, usize) {
        let show_range = width >= RANGE_MIN_WIDTH;
        let typing = matches!(self.mode, Mode::Typing(_));
        let mut selected_line = 0;
        let mut lines = Vec::new();
        for (n, row) in rows().into_iter().enumerate() {
            let i = match row {
                Row::Group(label) => {
                    lines.push(Line::styled(format!(" {label}"), theme::heading()));
                    continue;
                }
                Row::Setting(i) => i,
            };
            let setting = &SETTINGS[i];
            let selected = i == self.selected;
            if selected {
                selected_line = n;
            }
            let pad = LABEL_WIDTH.saturating_sub(setting.label.width());
            let value_style = if setting.is_default(&self.config) {
                theme::strong()
            } else {
                theme::accent()
            };
            let value = if selected && typing {
                " ".repeat(9)
            } else {
                format!("◀ {:>4} ▶", setting.value(&self.config))
            };
            let mut spans = vec![
                Span::styled(if selected { " › " } else { "   " }, theme::heading()),
                Span::styled(
                    format!("{}{}", setting.label, " ".repeat(pad)),
                    theme::strong(),
                ),
                Span::styled(value, value_style),
            ];
            if show_range {
                spans.push(Span::styled(
                    format!("   {}", setting.range()),
                    theme::dim(),
                ));
            }
            let mut line = Line::from(spans);
            if selected && !typing {
                line = line.style(theme::selected());
            }
            lines.push(line);
        }
        (lines, selected_line)
    }

    fn info_lines(&self, width: usize) -> Vec<Line<'static>> {
        let setting = self.setting();
        let wrapped = |text: &str, style| {
            widgets::wrap_text(text, width)
                .into_iter()
                .map(move |l| Line::styled(l, style))
        };
        let mut lines = Vec::new();
        match &self.mode {
            Mode::ConfirmResetAll => lines.push(Line::from(vec![
                Span::styled("Reset every setting? ", theme::bad()),
                Span::styled("y", theme::heading()),
                Span::styled(" yes   any other key: no", theme::dim()),
            ])),
            Mode::Typing(_) => lines.extend(wrapped(
                &format!("Type a number ({}), then Enter.", setting.range()),
                theme::dim(),
            )),
            Mode::Browse => lines.extend(wrapped(
                &format!("{} Default: {}", setting.help, setting.default_value()),
                theme::dim(),
            )),
        }
        if setting.key == "daily_goal" {
            let reachable = self.reachable_today();
            if i64::from(self.config.daily_goal) > reachable {
                let words = if reachable == 1 { "word" } else { "words" };
                lines.extend(wrapped(
                    &format!(
                        "With your word list and daily_new_limit, at most {reachable} {words} can be practised today."
                    ),
                    theme::warn(),
                ));
            }
        }
        lines.push(Line::from(vec![
            Span::styled("Saved to ", theme::dim()),
            Span::raw(self.config_file.clone()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Data     ", theme::dim()),
            Span::raw(self.data_dir.clone()),
        ]));
        lines
    }
}

impl View for Settings {
    fn title(&self) -> String {
        "Settings".to_string()
    }

    fn hints(&self) -> Vec<(&'static str, &'static str)> {
        match self.mode {
            Mode::Browse => vec![
                ("↑↓", "select"),
                ("←→", "adjust"),
                ("PgUp/PgDn", "±10"),
                ("Enter", "type"),
                ("r", "reset"),
                ("R", "reset all"),
                ("q", "back"),
            ],
            Mode::Typing(_) => vec![("Enter", "save"), ("↑↓", "cancel")],
            Mode::ConfirmResetAll => vec![("y", "reset all"), ("any key", "cancel")],
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let info = self.info_lines(area.width as usize);
        // The list keeps at least three rows; the details shrink first.
        let info_height = (info.len() as u16 + 1).min(area.height.saturating_sub(3));
        let [list_area, info_area] =
            Layout::vertical([Constraint::Min(3), Constraint::Length(info_height)]).areas(area);

        let (lines, selected_line) = self.list_lines(area.width);
        // Show the group heading too when scrolling up to its first setting.
        let first_visible = match selected_line.checked_sub(1) {
            Some(above) if matches!(rows()[above], Row::Group(_)) => above,
            _ => selected_line,
        };
        self.offset = widgets::scroll_to(
            self.offset.min(first_visible),
            selected_line,
            list_area.height as usize,
            lines.len(),
        );
        frame.render_widget(
            Paragraph::new(lines).scroll((self.offset as u16, 0)),
            list_area,
        );

        if let Mode::Typing(input) = &self.mode {
            let y = list_area.y as usize + selected_line - self.offset;
            if y < list_area.bottom() as usize {
                let row = Rect {
                    x: list_area.x + VALUE_X,
                    y: y as u16,
                    width: list_area.width.saturating_sub(VALUE_X),
                    height: 1,
                };
                input.render(frame, row, Span::styled("› ", theme::heading()));
            }
        }

        if info_height > 0 {
            let block = Block::new()
                .borders(Borders::TOP)
                .border_style(theme::dim());
            let inner = block.inner(info_area);
            frame.render_widget(block, info_area);
            frame.render_widget(Paragraph::new(info), inner);
        }
    }

    fn handle_key(&mut self, ctx: &mut Context, key: KeyEvent, fx: &mut Fx) -> Result<()> {
        match self.mode {
            Mode::Browse => self.browse_key(ctx, key, fx),
            Mode::Typing(_) => self.typing_key(ctx, key, fx),
            Mode::ConfirmResetAll => {
                self.mode = Mode::Browse;
                if key.code == KeyCode::Char('y') {
                    self.change(ctx, fx, |config| *config = Config::default());
                }
            }
        }
        Ok(())
    }

    fn interrupt(&mut self, _ctx: &mut Context, _fx: &mut Fx) -> Result<bool> {
        if matches!(self.mode, Mode::Browse) {
            return Ok(false);
        }
        self.mode = Mode::Browse;
        Ok(true)
    }
}
