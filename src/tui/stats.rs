use anyhow::Result;
use chrono::Utc;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui::Frame;

use super::{theme, widgets, Fx, Target, View};
use crate::commands::Context;
use crate::learning::Status;
use crate::storage;
use crate::time::{describe_due, format_local, local_day_start};

pub struct Stats {
    stats: storage::Stats,
    scroll: usize,
    /// Lines that do not fit below the scroll position, as of the last draw.
    max_scroll: usize,
}

impl Stats {
    pub fn load(ctx: &Context) -> Result<Self> {
        let now = Utc::now();
        Ok(Self {
            stats: ctx.db.stats(now, local_day_start(now))?,
            scroll: 0,
            max_scroll: 0,
        })
    }
}

fn percent(part: i64, whole: i64) -> String {
    if whole == 0 {
        "-".to_string()
    } else {
        format!("{}%", part * 100 / whole)
    }
}

fn status_style(status: Status) -> Style {
    match status {
        Status::New => theme::accent(),
        Status::Learning => theme::warn(),
        Status::Review => theme::strong(),
        Status::Mastered => theme::good(),
    }
}

impl View for Stats {
    fn title(&self) -> String {
        "Stats".to_string()
    }

    fn hints(&self) -> Vec<(&'static str, &'static str)> {
        if self.max_scroll > 0 {
            vec![("↑↓", "scroll"), ("q", "back")]
        } else {
            vec![("q", "back")]
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let s = &self.stats;
        let row = |label: &'static str, value: String| {
            Line::from(vec![
                Span::styled(format!("{label:<14}"), theme::dim()),
                Span::styled(value, theme::strong()),
            ])
        };
        let mut lines = vec![row("Saved words", s.total_words.to_string()), Line::raw("")];
        let bar_width = (area.width as usize).saturating_sub(40).clamp(10, 40);
        for (status, n) in &s.status_counts {
            let ratio = if s.total_words > 0 {
                *n as f64 / s.total_words as f64
            } else {
                0.0
            };
            let filled = (ratio * bar_width as f64).round() as usize;
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<12}", status.as_str()), status_style(*status)),
                Span::styled("█".repeat(filled), status_style(*status)),
                Span::styled("░".repeat(bar_width - filled), theme::dim()),
                Span::styled(format!("  {n}"), theme::strong()),
            ]));
        }
        lines.push(Line::raw(""));
        lines.push(row("Due now", s.due_now.to_string()));
        lines.push(row(
            "Today",
            format!(
                "{} answers · {} correct · {} new words",
                s.attempts_today,
                percent(s.correct_today, s.attempts_today),
                s.new_words_today
            ),
        ));
        lines.push(row(
            "All time",
            format!(
                "{} answers · {} correct",
                s.total_attempts,
                percent(s.total_correct, s.total_attempts)
            ),
        ));
        if let Some(next) = s.next_due {
            lines.push(row(
                "Next review",
                format!(
                    "{} ({})",
                    format_local(next),
                    describe_due(next, Utc::now())
                ),
            ));
        }
        let accuracy = if s.total_attempts > 0 {
            s.total_correct as f64 / s.total_attempts as f64
        } else {
            0.0
        };
        lines.push(Line::raw(""));
        let mut accuracy_line = vec![Span::styled(format!("{:<14}", "Accuracy"), theme::dim())];
        accuracy_line.extend(widgets::progress_bar(accuracy, bar_width));
        lines.push(Line::from(accuracy_line));

        let rows = lines.len() as u16;
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(theme::dim())
            .title(Span::styled(" Progress ", theme::heading()))
            .padding(widgets::box_padding(3, rows, area.height));
        self.max_scroll = lines
            .len()
            .saturating_sub(block.inner(area).height as usize);
        self.scroll = self.scroll.min(self.max_scroll);
        frame.render_widget(
            Paragraph::new(lines)
                .block(block)
                .scroll((self.scroll as u16, 0)),
            area,
        );
    }

    fn handle_key(&mut self, _ctx: &mut Context, key: KeyEvent, fx: &mut Fx) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll = (self.scroll + 1).min(self.max_scroll);
            }
            KeyCode::Char('q') | KeyCode::Enter | KeyCode::Backspace => fx.go(Target::Home),
            _ => {}
        }
        Ok(())
    }
}
