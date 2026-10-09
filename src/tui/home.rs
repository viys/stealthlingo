use anyhow::Result;
use chrono::{DateTime, Utc};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Padding, Paragraph};
use ratatui::Frame;

use super::{theme, Fx, Target, View};
use crate::cli::StudyMode;
use crate::commands::study::SessionRequest;
use crate::commands::Context;
use crate::learning::Status;
use crate::time::{describe_due, local_day_start};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Item {
    Review,
    Study(StudyMode),
    Lookup,
    Words,
    Stats,
    Quit,
}

const MENU: [(char, &str, Item); 8] = [
    ('r', "Review due words", Item::Review),
    ('s', "Study · flashcards", Item::Study(StudyMode::Memory)),
    ('p', "Spelling practice", Item::Study(StudyMode::Spelling)),
    ('l', "Listening practice", Item::Study(StudyMode::Listening)),
    ('/', "Look up a word", Item::Lookup),
    ('w', "Word list", Item::Words),
    ('t', "Stats", Item::Stats),
    ('q', "Quit", Item::Quit),
];

pub struct Home {
    saved: i64,
    due: i64,
    new_available: i64,
    answers_today: i64,
    correct_today: i64,
    next_due: Option<DateTime<Utc>>,
    selected: usize,
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
            next_due: stats.next_due,
            // Today's practice: reviews when something is due, otherwise new words.
            selected: if stats.due_now > 0 { 0 } else { 1 },
        };
        Ok(home)
    }

    fn activate(&self, item: Item, fx: &mut Fx) {
        match item {
            Item::Review => fx.go(Target::Session(SessionRequest::review(None, None))),
            Item::Study(mode) => fx.go(Target::Session(SessionRequest {
                mode,
                due_only: false,
                minutes: None,
                count: None,
            })),
            Item::Lookup => fx.go(Target::Lookup),
            Item::Words => fx.go(Target::Words),
            Item::Stats => fx.go(Target::Stats),
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
        vec![
            row(
                "Due now",
                self.due.to_string(),
                if self.due > 0 {
                    theme::warn()
                } else {
                    theme::good()
                },
            ),
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
        ]
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
        let summary = self.summary();
        let [top, _, menu] = Layout::vertical([
            Constraint::Length(summary.len() as u16 + 4),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .areas(area);
        frame.render_widget(
            Paragraph::new(summary).block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(theme::dim())
                    .title(Span::styled(" Today ", theme::heading()))
                    .padding(Padding::new(2, 2, 1, 1)),
            ),
            top,
        );

        let lines: Vec<Line> = MENU
            .iter()
            .enumerate()
            .map(|(i, (key, label, item))| {
                let selected = i == self.selected;
                let marker = if selected { " › " } else { "   " };
                let mut line = Line::from(vec![
                    Span::styled(marker, theme::heading()),
                    Span::styled(format!("{key}  "), theme::heading()),
                    Span::styled(format!("{label:<22}"), theme::strong()),
                    Span::styled(self.detail(*item), theme::dim()),
                ]);
                if selected {
                    line = line.style(theme::selected());
                }
                line
            })
            .collect();
        frame.render_widget(Paragraph::new(lines), menu);
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
