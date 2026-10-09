use std::time::Instant;

use anyhow::Result;
use chrono::{DateTime, Utc};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Padding, Paragraph};
use ratatui::Frame;

use super::widgets::{self, headline, is_ctrl, meaning_lines, LineInput};
use super::{theme, Fx, Target, Tone, View, Voice};
use crate::cli::StudyMode;
use crate::commands::study::SessionRequest;
use crate::commands::Context;
use crate::learning::session::{
    gave_up, judge_spelling, memory_answer, Answer, Session, SessionPlan,
};
use crate::learning::spelling::diff_chars;
use crate::learning::{Grade, Status};
use crate::storage::StudyItem;
use crate::time::describe_due;

enum Phase {
    /// Flashcard front: the word only.
    Front,
    /// Flashcard back: meaning revealed, waiting for a rating.
    Back,
    Typing(LineInput),
    Feedback {
        answer: Answer,
        next_due: DateTime<Utc>,
    },
}

struct Card {
    item: StudyItem,
    asked_at: Instant,
    phase: Phase,
    voice: Voice,
}

struct LastResult {
    word: String,
    answer: Answer,
    next_due: DateTime<Utc>,
}

enum Stage {
    Card(Box<Card>),
    Summary {
        reason: &'static str,
        next: Option<String>,
    },
}

pub struct Study {
    request: SessionRequest,
    session: Session,
    stage: Stage,
    last: Option<LastResult>,
}

impl Study {
    pub fn start(
        ctx: &Context,
        request: SessionRequest,
        plan: SessionPlan,
        queue: Vec<StudyItem>,
        fx: &mut Fx,
    ) -> Self {
        let mut study = Self {
            request,
            session: Session::new(plan, queue),
            stage: Stage::Summary {
                reason: "",
                next: None,
            },
            last: None,
        };
        study.advance(ctx, fx);
        study
    }

    fn mode(&self) -> StudyMode {
        self.session.plan().mode
    }

    fn advance(&mut self, ctx: &Context, fx: &mut Fx) {
        let Some(item) = self.session.next_item() else {
            self.finish(ctx);
            return;
        };
        let phase = match self.mode() {
            StudyMode::Memory => Phase::Front,
            StudyMode::Spelling | StudyMode::Listening => Phase::Typing(LineInput::default()),
        };
        let mut voice = Voice::default();
        if self.mode() == StudyMode::Listening {
            voice.play(&item.entry, &self.session.plan().accent, fx);
        }
        self.stage = Stage::Card(Box::new(Card {
            item,
            asked_at: Instant::now(),
            phase,
            voice,
        }));
    }

    fn finish(&mut self, ctx: &Context) {
        let summary = self.session.summary();
        let reason = if summary.time_up {
            "Time's up"
        } else if summary.stopped_early {
            "Session ended"
        } else if self.session.queue_is_empty() {
            "All done for now"
        } else {
            "Session complete"
        };
        let now = Utc::now();
        let next = match ctx.db.count_due(now) {
            Ok(n) if n > 0 => Some(format!(
                "{n} {} due now.",
                if n == 1 { "word is" } else { "words are" }
            )),
            _ => ctx
                .db
                .next_due_at()
                .ok()
                .flatten()
                .map(|next| format!("Next review {}.", describe_due(next, now))),
        };
        self.stage = Stage::Summary { reason, next };
    }

    fn record(&mut self, ctx: &mut Context, answer: Answer) -> Result<DateTime<Utc>> {
        let Stage::Card(card) = &self.stage else {
            unreachable!("answers are only recorded for a card");
        };
        let next = self.session.record(
            &mut ctx.db,
            card.item.clone(),
            &answer,
            card.asked_at.elapsed(),
        )?;
        self.last = Some(LastResult {
            word: card.item.entry.word.clone(),
            answer,
            next_due: next.due_at,
        });
        Ok(next.due_at)
    }

    fn card_key(&mut self, ctx: &mut Context, key: KeyEvent, fx: &mut Fx) -> Result<()> {
        let mode = self.mode();
        let accent = self.session.plan().accent.clone();
        let Stage::Card(card) = &mut self.stage else {
            return Ok(());
        };
        match &mut card.phase {
            Phase::Front => match key.code {
                KeyCode::Char(' ') | KeyCode::Enter => card.phase = Phase::Back,
                KeyCode::Char('a') => card.voice.play(&card.item.entry, &accent, fx),
                KeyCode::Char('s') => {
                    self.session.skip();
                    self.advance(ctx, fx);
                }
                KeyCode::Char('q') => self.end(ctx),
                _ => {}
            },
            Phase::Back => {
                let grade = match key.code {
                    KeyCode::Char('1') => Some(Grade::Again),
                    KeyCode::Char('2') => Some(Grade::Hard),
                    KeyCode::Char('3') => Some(Grade::Good),
                    KeyCode::Char('4') => Some(Grade::Easy),
                    KeyCode::Char('a') => {
                        card.voice.play(&card.item.entry, &accent, fx);
                        None
                    }
                    KeyCode::Char('q') => {
                        self.end(ctx);
                        None
                    }
                    _ => None,
                };
                if let Some(grade) = grade {
                    self.record(ctx, memory_answer(grade))?;
                    self.advance(ctx, fx);
                }
            }
            Phase::Typing(input) => {
                if is_ctrl(&key, 'r') && mode == StudyMode::Listening {
                    card.voice.play(&card.item.entry, &accent, fx);
                } else if is_ctrl(&key, 'n') {
                    self.session.skip();
                    self.advance(ctx, fx);
                } else if key.code == KeyCode::Tab {
                    let next_due = self.record(ctx, gave_up())?;
                    self.show_feedback(next_due);
                } else if key.code == KeyCode::Enter {
                    let typed = input.text().trim().to_string();
                    if typed.is_empty() {
                        if mode == StudyMode::Listening {
                            card.voice.replay(&card.item.entry, &accent, fx);
                        } else {
                            fx.flash(Tone::Info, "Type the word, or press Tab to see the answer.");
                        }
                    } else {
                        let answer = judge_spelling(&card.item.entry.word, &typed);
                        let next_due = self.record(ctx, answer)?;
                        self.show_feedback(next_due);
                    }
                } else {
                    input.handle(key);
                }
            }
            Phase::Feedback { .. } => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => self.advance(ctx, fx),
                KeyCode::Char('a') => card.voice.play(&card.item.entry, &accent, fx),
                KeyCode::Char('q') => self.end(ctx),
                _ => {}
            },
        }
        Ok(())
    }

    fn show_feedback(&mut self, next_due: DateTime<Utc>) {
        if let (Stage::Card(card), Some(last)) = (&mut self.stage, &self.last) {
            card.phase = Phase::Feedback {
                answer: last.answer.clone(),
                next_due,
            };
        }
    }

    /// Ends the session; the current question is not recorded.
    fn end(&mut self, ctx: &Context) {
        self.session.stop();
        self.finish(ctx);
    }

    fn progress_line(&self, width: usize) -> Line<'static> {
        let summary = self.session.summary();
        let done = summary.answered + summary.skipped;
        let total = done + self.session.remaining();
        let mut tail = format!("  {done}/{total}");
        if let Some(left) = self.session.time_left() {
            tail.push_str(&format!("  ·  {} left", widgets::clock(left.as_secs())));
        }
        let label = format!("{}  ", self.mode().label());
        let bar_width = width
            .saturating_sub(label.chars().count() + tail.chars().count())
            .clamp(10, 40);
        let mut spans = vec![Span::styled(label, theme::heading())];
        spans.extend(widgets::progress_bar(
            done as f64 / total.max(1) as f64,
            bar_width,
        ));
        spans.push(Span::styled(tail, theme::dim()));
        Line::from(spans)
    }

    /// The card's text; typing phases leave two rows below it for the answer.
    fn card_lines(&self, card: &Card, width: usize) -> Vec<Line<'static>> {
        let entry = &card.item.entry;
        let note = card.item.note.as_deref();
        let mut lines = Vec::new();
        match &card.phase {
            Phase::Front | Phase::Back => {
                lines.extend([headline(entry), status_line(&card.item), Line::raw("")]);
                if matches!(card.phase, Phase::Back) {
                    lines.extend(meaning_lines(entry, note, false, width));
                } else {
                    lines.push(Line::styled(
                        "Recall the meaning, then reveal it.",
                        theme::dim(),
                    ));
                }
            }
            Phase::Typing(_) => {
                if self.mode() == StudyMode::Listening {
                    lines.push(Line::from(vec![
                        Span::styled("♪  ", theme::heading()),
                        Span::styled("Listen and type the word you hear.", theme::strong()),
                    ]));
                    if let Some(note) = note {
                        lines.push(Line::raw(""));
                        lines.extend(widgets::labelled(
                            "note",
                            theme::label(),
                            6,
                            note,
                            theme::warn(),
                            width,
                        ));
                    }
                } else {
                    lines.extend(meaning_lines(entry, note, true, width));
                }
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![
                    Span::styled("hint  ", theme::label()),
                    Span::styled(letter_hint(&entry.word), theme::dim()),
                ]));
            }
            Phase::Feedback { answer, next_due } => {
                lines.extend(feedback_lines(entry, answer));
                lines.push(Line::raw(""));
                lines.extend(meaning_lines(entry, note, false, width));
                lines.push(Line::raw(""));
                lines.push(Line::styled(
                    format!("Next review {}", describe_due(*next_due, Utc::now())),
                    theme::dim(),
                ));
            }
        }
        lines
    }

    /// Draws the card in a box as tall as its content and returns the box.
    fn render_card(&self, card: &Card, frame: &mut Frame, area: Rect) -> Rect {
        let width = Block::bordered()
            .padding(Padding::horizontal(3))
            .inner(area)
            .width as usize;
        let lines = self.card_lines(card, width);
        let input_rows = if matches!(card.phase, Phase::Typing(_)) {
            2
        } else {
            0
        };
        let used = lines.len() as u16;
        let padding = widgets::box_padding(3, used + input_rows, area.height);
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(theme::dim())
            .padding(padding);
        let area = Rect {
            height: widgets::box_height(used + input_rows, padding, area.height),
            ..area
        };
        let inner = block.inner(area);
        // The answer field stays on screen even when the text has to be cut.
        let text = Rect {
            height: inner.height.saturating_sub(input_rows),
            ..inner
        };
        frame.render_widget(block, area);
        frame.render_widget(Paragraph::new(lines), text);
        if let Phase::Typing(input) = &card.phase {
            if inner.height >= input_rows {
                let row = Rect {
                    y: text.y + used.min(text.height) + 1,
                    height: 1,
                    ..inner
                };
                input.render(frame, row, Span::styled("› ", theme::heading()));
            }
        }
        area
    }

    fn last_line(&self) -> Line<'static> {
        let Some(last) = &self.last else {
            return Line::raw("");
        };
        let (mark, style) = if last.answer.is_correct {
            ("✓", theme::good())
        } else {
            ("✗", theme::bad())
        };
        Line::from(vec![
            Span::styled(format!("{mark} "), style),
            Span::styled(last.word.clone(), theme::strong()),
            Span::styled(
                format!(
                    "  ·  {}  ·  next {}",
                    last.answer.grade.as_str(),
                    describe_due(last.next_due, Utc::now())
                ),
                theme::dim(),
            ),
        ])
    }

    fn render_summary(&self, reason: &str, next: Option<&str>, frame: &mut Frame, area: Rect) {
        let summary = self.session.summary();
        let accuracy = (summary.correct * 100)
            .checked_div(summary.answered)
            .map_or("-".to_string(), |p| format!("{p}%"));
        let row = |label: &'static str, value: String, style: Style| {
            Line::from(vec![
                Span::styled(format!("{label:<11}"), theme::dim()),
                Span::styled(value, style),
            ])
        };
        let mut lines = vec![
            Line::styled(reason.to_string(), theme::heading()),
            Line::raw(""),
            row(
                "Answered",
                format!(
                    "{}   ·   {} correct ({accuracy})",
                    summary.answered, summary.correct
                ),
                theme::strong(),
            ),
        ];
        if summary.skipped > 0 {
            lines.push(row("Skipped", summary.skipped.to_string(), theme::strong()));
        }
        lines.push(row(
            "Time",
            widgets::clock(self.session.elapsed().as_secs()),
            theme::strong(),
        ));
        if !summary.missed.is_empty() {
            lines.push(row("Missed", summary.missed.join(", "), theme::bad()));
        }
        if summary.answered == 0 {
            lines.push(Line::raw(""));
            lines.push(Line::styled("No answers recorded.", theme::dim()));
        }
        if let Some(next) = next {
            lines.push(Line::raw(""));
            lines.push(Line::styled(next.to_string(), theme::dim()));
        }
        let rows = lines.len() as u16;
        let padding = widgets::box_padding(3, rows, area.height);
        let area = Rect {
            height: widgets::box_height(rows, padding, area.height),
            ..area
        };
        frame.render_widget(
            Paragraph::new(lines).block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .border_style(theme::dim())
                    .title(Span::styled(
                        format!(" {} ", self.request.title()),
                        theme::heading(),
                    ))
                    .padding(padding),
            ),
            area,
        );
    }
}

fn status_line(item: &StudyItem) -> Line<'static> {
    let text = match item.schedule.status {
        Status::New => "new word",
        status => status.as_str(),
    };
    Line::styled(text, theme::dim())
}

fn letter_hint(word: &str) -> String {
    let letters = word.chars().filter(|c| c.is_alphabetic()).count();
    match word.split_whitespace().count() {
        n if n > 1 => format!("{n} words, {letters} letters"),
        _ => format!("{letters} letters"),
    }
}

/// Verdict, then for a wrong answer the typed text and the answer with the
/// differing letters marked.
fn feedback_lines(entry: &crate::dictionary::Entry, answer: &Answer) -> Vec<Line<'static>> {
    if answer.is_correct {
        return vec![
            Line::styled("✓ Correct", theme::good().add_modifier(Modifier::BOLD)),
            Line::raw(""),
            headline(entry),
        ];
    }
    let mut lines = vec![Line::styled(
        "✗ Not quite",
        theme::bad().add_modifier(Modifier::BOLD),
    )];
    lines.push(Line::raw(""));
    match answer.submitted.as_deref() {
        Some(submitted) => {
            let (expected, typed) = diff_chars(&entry.word, submitted);
            let mut you = vec![Span::styled("you typed  ", theme::dim())];
            you.extend(typed.into_iter().map(|(c, ok)| {
                if ok {
                    Span::raw(c.to_string())
                } else {
                    Span::styled(
                        c.to_string(),
                        theme::bad().add_modifier(Modifier::CROSSED_OUT),
                    )
                }
            }));
            let mut correct = vec![Span::styled("answer     ", theme::dim())];
            correct.extend(expected.into_iter().map(|(c, ok)| {
                if ok {
                    Span::styled(c.to_string(), theme::strong())
                } else {
                    Span::styled(
                        c.to_string(),
                        theme::good().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                    )
                }
            }));
            lines.push(Line::from(you));
            lines.push(Line::from(correct));
            lines.push(Line::raw(""));
            lines.push(headline(entry));
        }
        None => lines.push(headline(entry)),
    }
    lines
}

impl View for Study {
    fn title(&self) -> String {
        self.request.title().to_string()
    }

    fn hints(&self) -> Vec<(&'static str, &'static str)> {
        let Stage::Card(card) = &self.stage else {
            return vec![("Enter", "home"), ("r", "another round"), ("q", "quit")];
        };
        let audio = card
            .voice
            .hint(&card.item.entry, &self.session.plan().accent);
        match &card.phase {
            Phase::Front => vec![
                ("Space", "reveal"),
                ("a", audio),
                ("s", "skip"),
                ("q", "end"),
            ],
            Phase::Back => vec![
                ("1", "again"),
                ("2", "hard"),
                ("3", "good"),
                ("4", "easy"),
                ("a", audio),
                ("q", "end"),
            ],
            Phase::Typing(_) if self.mode() == StudyMode::Listening => vec![
                ("Enter", "submit / replay"),
                ("Tab", "show answer"),
                ("^R", if audio == "audio" { "replay" } else { audio }),
                ("^N", "skip"),
                ("^C", "end"),
            ],
            Phase::Typing(_) => vec![
                ("Enter", "submit"),
                ("Tab", "show answer"),
                ("^N", "skip"),
                ("^C", "end"),
            ],
            Phase::Feedback { .. } => vec![("Enter", "next"), ("a", audio), ("q", "end")],
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        match &self.stage {
            Stage::Card(card) => {
                let [progress, _, body] = Layout::vertical([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(6),
                ])
                .areas(area);
                frame.render_widget(
                    Paragraph::new(self.progress_line(area.width as usize)),
                    progress,
                );
                let card_area = self.render_card(card, frame, body);
                let below = card_area.bottom() + 1;
                if !matches!(card.phase, Phase::Feedback { .. }) && below < body.bottom() {
                    let last = Rect {
                        y: below,
                        height: 1,
                        ..body
                    };
                    frame.render_widget(Paragraph::new(self.last_line()), last);
                }
            }
            Stage::Summary { reason, next } => {
                self.render_summary(reason, next.as_deref(), frame, area);
            }
        }
    }

    fn handle_key(&mut self, ctx: &mut Context, key: KeyEvent, fx: &mut Fx) -> Result<()> {
        if matches!(self.stage, Stage::Card(_)) {
            return self.card_key(ctx, key, fx);
        }
        match key.code {
            KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('h') => fx.go(Target::Home),
            KeyCode::Char('r') => fx.go(Target::Session(self.request)),
            KeyCode::Char('q') => fx.quit(),
            _ => {}
        }
        Ok(())
    }

    fn interrupt(&mut self, ctx: &mut Context, _fx: &mut Fx) -> Result<bool> {
        if matches!(self.stage, Stage::Card(_)) {
            self.end(ctx);
            return Ok(true);
        }
        Ok(false)
    }

    fn set_paused(&mut self, paused: bool) {
        if paused {
            self.session.pause();
        } else {
            self.session.resume();
        }
    }
}
