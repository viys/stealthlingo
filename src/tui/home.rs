use anyhow::Result;
use chrono::{DateTime, Utc};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui::Frame;

use super::{theme, widgets, Fx, Target, View};
use crate::cli::{StudyArgs, StudyMode};
use crate::commands::stats::goal_progress;
use crate::commands::study::SessionRequest;
use crate::commands::Context;
use crate::learning::Status;
use crate::time::{describe_due, local_day_start};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Item {
    Review,
    Study(StudyMode),
    Mistakes,
    Lookup,
    Words,
    Stats,
    Settings,
    Quit,
}

const MENU: [(char, &str, Item); 12] = [
    ('r', "Review due words", Item::Review),
    ('s', "Study · flashcards", Item::Study(StudyMode::Memory)),
    ('p', "Spelling practice", Item::Study(StudyMode::Spelling)),
    ('m', "Missing letters", Item::Study(StudyMode::Letters)),
    ('l', "Listening practice", Item::Study(StudyMode::Listening)),
    ('x', "Mixed practice", Item::Study(StudyMode::Mixed)),
    ('e', "Retry mistakes", Item::Mistakes),
    ('/', "Look up a word", Item::Lookup),
    ('w', "Word list", Item::Words),
    ('t', "Stats", Item::Stats),
    ('c', "Settings", Item::Settings),
    ('q', "Quit", Item::Quit),
];

/// Body width from which a menu that does not fit is split into two columns.
const TWO_COLUMNS_WIDTH: u16 = 84;

pub struct Home {
    saved: i64,
    due: i64,
    new_available: i64,
    answers_today: i64,
    correct_today: i64,
    words_today: i64,
    daily_goal: u32,
    next_due: Option<DateTime<Utc>>,
    selected: usize,
    /// First menu row shown when the menu does not fit.
    menu_offset: usize,
}

impl Home {
    pub fn load(ctx: &Context) -> Result<Self> {
        let now = Utc::now();
        let stats = ctx.db.stats(now, local_day_start(now))?;
        let new_ready = stats
            .status_counts
            .iter()
            .find(|(s, _)| *s == Status::New)
            .map_or(0, |(_, n)| *n);
        let allowance = (i64::from(ctx.config.daily_new_limit) - stats.new_words_today).max(0);
        let home = Self {
            saved: stats.total_words,
            due: stats.due_now,
            new_available: new_ready.min(allowance),
            answers_today: stats.attempts_today,
            correct_today: stats.correct_today,
            words_today: stats.words_today,
            daily_goal: ctx.config.daily_goal,
            next_due: stats.next_due,
            // Today's practice: reviews when something is due, otherwise new words.
            selected: if stats.due_now > 0 { 0 } else { 1 },
            menu_offset: 0,
        };
        Ok(home)
    }

    fn activate(&self, item: Item, fx: &mut Fx) {
        match item {
            Item::Review => fx.go(Target::Session(
                SessionRequest::review(None, None).toward_goal(),
            )),
            Item::Study(mode) => fx.go(Target::Session(
                SessionRequest {
                    mode,
                    ..SessionRequest::study(&StudyArgs::default())
                }
                .toward_goal(),
            )),
            Item::Mistakes => fx.go(Target::Session(
                SessionRequest {
                    mode: StudyMode::Mixed,
                    mistakes: true,
                    ..SessionRequest::study(&StudyArgs::default())
                }
                .toward_goal(),
            )),
            Item::Lookup => fx.go(Target::Lookup),
            Item::Words => fx.go(Target::Words),
            Item::Stats => fx.go(Target::Stats),
            Item::Settings => fx.go(Target::Settings),
            Item::Quit => fx.quit(),
        }
    }

    fn detail(&self, item: Item) -> String {
        match item {
            Item::Review if self.due > 0 => format!("{} due", self.due),
            Item::Review => "nothing due".to_string(),
            Item::Study(_) if self.new_available > 0 => {
                format!("{} new available", self.new_available)
            }
            Item::Words => format!("{} saved", self.saved),
            _ => String::new(),
        }
    }

    fn summary(&self) -> Vec<Line<'static>> {
        let row = |label: &'static str, value: String, style| {
            Line::from(vec![
                Span::styled(format!("{label:<13}"), theme::dim()),
                Span::styled(value, style),
            ])
        };
        if self.saved == 0 {
            return vec![
                Line::styled("Your word list is empty.", theme::strong()),
                Line::from(vec![
                    Span::raw("Press "),
                    Span::styled("/", theme::heading()),
                    Span::raw(" to look a word up, then "),
                    Span::styled("s", theme::heading()),
                    Span::raw(" to save it for practice."),
                ]),
            ];
        }
        let now = Utc::now();
        let accuracy = if self.answers_today > 0 {
            format!(
                " · {}% correct",
                self.correct_today * 100 / self.answers_today
            )
        } else {
            String::new()
        };
        let mut lines = Vec::new();
        if self.daily_goal > 0 {
            let style = if self.words_today >= i64::from(self.daily_goal) {
                theme::good()
            } else {
                theme::strong()
            };
            let mut goal = row(
                "Daily goal",
                goal_progress(self.words_today, self.daily_goal),
                style,
            );
            goal.spans.push(Span::styled("   c change", theme::dim()));
            lines.push(goal);
        }
        lines.extend([
            row("Due now", self.due.to_string(), self.due_style()),
            row(
                "New today",
                format!("{} available", self.new_available),
                theme::strong(),
            ),
            row(
                "Answered",
                format!("{} today{accuracy}", self.answers_today),
                theme::strong(),
            ),
            row(
                "Next review",
                self.next_due
                    .map_or("-".to_string(), |next| describe_due(next, now)),
                theme::strong(),
            ),
        ]);
        lines
    }

    /// The numbers of [`Self::summary`] on one line, for short terminals.
    fn compact_summary(&self) -> Vec<Line<'static>> {
        if self.saved == 0 {
            return self.summary();
        }
        let next = self
            .next_due
            .map_or("-".to_string(), |next| describe_due(next, Utc::now()));
        let mut spans = Vec::new();
        if self.daily_goal > 0 {
            spans.extend([
                Span::styled("Goal ", theme::dim()),
                Span::styled(
                    format!("{}/{}", self.words_today, self.daily_goal),
                    theme::strong(),
                ),
                Span::styled(" · ", theme::dim()),
            ]);
        }
        spans.extend([
            Span::styled("Due ", theme::dim()),
            Span::styled(self.due.to_string(), self.due_style()),
            Span::styled(" · New ", theme::dim()),
            Span::styled(self.new_available.to_string(), theme::strong()),
            Span::styled(" · Answered ", theme::dim()),
            Span::styled(self.answers_today.to_string(), theme::strong()),
            Span::styled(" · Next ", theme::dim()),
            Span::styled(next, theme::strong()),
        ]);
        vec![Line::from(spans)]
    }

    fn due_style(&self) -> Style {
        if self.due > 0 {
            theme::warn()
        } else {
            theme::good()
        }
    }
}

impl View for Home {
    fn title(&self) -> String {
        "Home".to_string()
    }

    fn hints(&self) -> Vec<(&'static str, &'static str)> {
        vec![("↑↓", "move"), ("Enter", "open"), ("q", "quit")]
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Short terminals get a tighter box, then a one-line summary, then a
        // menu in two columns when it is wide enough, or else a scrolling
        // menu, so the highlighted item is always on screen.
        let columns = if area.height < MENU.len() as u16 + 2 && area.width >= TWO_COLUMNS_WIDTH {
            2
        } else {
            1
        };
        let menu_rows = MENU.len().div_ceil(columns) as u16;
        let summary = self.summary();
        let rows = summary.len() as u16;
        let room = area.height.saturating_sub(menu_rows + 1);
        let top_height = if rows + 2 <= room {
            let padding = widgets::box_padding(2, rows, room);
            let height = widgets::box_height(rows, padding, room);
            frame.render_widget(
                Paragraph::new(summary).block(
                    Block::bordered()
                        .border_type(BorderType::Rounded)
                        .border_style(theme::dim())
                        .title(Span::styled(" Today ", theme::heading()))
                        .padding(padding),
                ),
                Rect { height, ..area },
            );
            height
        } else {
            let compact = self.compact_summary();
            let height = (compact.len() as u16).min(area.height);
            frame.render_widget(Paragraph::new(compact), Rect { height, ..area });
            height
        };
        let gap = u16::from(area.height > top_height + menu_rows);
        let menu = Rect {
            y: area.y + top_height + gap,
            height: area.height.saturating_sub(top_height + gap),
            ..area
        };
        let lines: Vec<Line> = MENU
            .iter()
            .enumerate()
            .map(|(i, (key, label, item))| {
                let selected = i == self.selected;
                let marker = if selected { " › " } else { "   " };
                let mut line = Line::from(vec![
                    Span::styled(marker, theme::heading()),
                    Span::styled(format!("{key}  "), theme::heading()),
                    Span::styled(format!("{label:<20}"), theme::strong()),
                    Span::styled(self.detail(*item), theme::dim()),
                ]);
                if selected {
                    line = line.style(theme::selected());
                }
                line
            })
            .collect();
        if columns == 2 {
            let [left, right] =
                Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(menu);
            let mut lines = lines;
            let second = lines.split_off(menu_rows as usize);
            frame.render_widget(Paragraph::new(lines), left);
            frame.render_widget(Paragraph::new(second), right);
            return;
        }
        self.menu_offset = widgets::scroll_to(
            self.menu_offset,
            self.selected,
            menu.height as usize,
            MENU.len(),
        );
        frame.render_widget(
            Paragraph::new(lines).scroll((self.menu_offset as u16, 0)),
            menu,
        );
    }

    fn handle_key(&mut self, _ctx: &mut Context, key: KeyEvent, fx: &mut Fx) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = (self.selected + MENU.len() - 1) % MENU.len();
            }
            KeyCode::Down | KeyCode::Char('j') => self.selected = (self.selected + 1) % MENU.len(),
            KeyCode::Enter | KeyCode::Char(' ') => self.activate(MENU[self.selected].2, fx),
            KeyCode::Char(c) => {
                let c = c.to_ascii_lowercase();
                if let Some(i) = MENU.iter().position(|(k, _, _)| *k == c) {
                    self.selected = i;
                    self.activate(MENU[i].2, fx);
                }
            }
            _ => {}
        }
        Ok(())
    }
}
