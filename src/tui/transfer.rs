//! Import / export paths from the settings screen.

use std::env;
use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::Local;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::widgets::LineInput;
use super::{theme, Fx, Target, Tone, View};
use crate::commands::io::{self, ExportResult};
use crate::commands::Context;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Start {
    Export,
    Import,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    Json,
    Csv,
}

impl Format {
    fn label(self) -> &'static str {
        match self {
            Self::Json => "JSON (full backup)",
            Self::Csv => "CSV (word list)",
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Csv => "csv",
        }
    }

    fn toggle(self) -> Self {
        match self {
            Self::Json => Self::Csv,
            Self::Csv => Self::Json,
        }
    }

    fn from_path(path: &Path) -> Option<Self> {
        match path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("json") => Some(Self::Json),
            Some("csv") => Some(Self::Csv),
            _ => None,
        }
    }
}

enum Phase {
    Edit,
    Confirm,
}

enum Kind {
    Export { format: Format },
    Import,
}

pub struct Transfer {
    kind: Kind,
    path: LineInput,
    phase: Phase,
    /// Where relative paths are resolved (the process working directory).
    cwd: String,
}

impl Transfer {
    pub fn new(start: Start) -> Self {
        let format = Format::Json;
        let cwd = env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| ".".to_string());
        Self {
            kind: match start {
                Start::Export => Kind::Export { format },
                Start::Import => Kind::Import,
            },
            // Export suggests today's filename; import starts empty so Enter
            // does not try to read a file that was never written.
            path: match start {
                Start::Export => LineInput::with_text(&default_path(format)),
                Start::Import => LineInput::default(),
            },
            phase: Phase::Edit,
            cwd,
        }
    }

    fn format_label(&self) -> &'static str {
        match self.kind {
            Kind::Export { format } => format.label(),
            Kind::Import => "from extension (.json / .csv)",
        }
    }

    fn toggle_format(&mut self) {
        let Kind::Export { format } = &mut self.kind else {
            return;
        };
        *format = format.toggle();
        let next = PathBuf::from(self.path.text()).with_extension(format.extension());
        self.path = LineInput::with_text(&next.to_string_lossy());
    }

    fn path_buf(&self) -> PathBuf {
        PathBuf::from(self.path.text().trim())
    }

    fn run_export(&mut self, ctx: &Context, fx: &mut Fx) {
        let file = self.path_buf();
        if file.as_os_str().is_empty() {
            fx.flash(Tone::Bad, "Enter a file path.");
            self.phase = Phase::Edit;
            return;
        }
        match io::export(ctx, &file) {
            Ok(result) => {
                fx.flash(Tone::Good, result.summary(&file));
                if let Kind::Export { format } = &mut self.kind {
                    *format = match &result {
                        ExportResult::Json { .. } => Format::Json,
                        ExportResult::Csv { .. } => Format::Csv,
                    };
                }
            }
            Err(err) => fx.flash(Tone::Bad, format!("{err:#}")),
        }
        self.phase = Phase::Edit;
    }

    fn run_import(&mut self, ctx: &mut Context, fx: &mut Fx) {
        let file = self.path_buf();
        if file.as_os_str().is_empty() {
            fx.flash(Tone::Bad, "Enter a file path.");
            self.phase = Phase::Edit;
            return;
        }
        match io::import(ctx, &file) {
            Ok(result) => fx.flash(Tone::Good, result.summary()),
            Err(err) => fx.flash(Tone::Bad, format!("{err:#}")),
        }
        self.phase = Phase::Edit;
    }

    fn confirm_prompt(&self) -> Option<Line<'static>> {
        if !matches!(self.phase, Phase::Confirm) {
            return None;
        }
        match &self.kind {
            Kind::Export { .. } => Some(Line::from(vec![
                Span::styled("Overwrite this file? ", theme::warn()),
                Span::styled("y/l", theme::heading()),
                Span::styled(" yes   any other key: no", theme::dim()),
            ])),
            Kind::Import => {
                let question = match Format::from_path(&self.path_buf()) {
                    Some(Format::Csv) => {
                        "Look up and save words from this list? May use the network. "
                    }
                    _ => "Merge backup into this library? ",
                };
                Some(Line::from(vec![
                    Span::styled(question, theme::warn()),
                    Span::styled("y/l", theme::heading()),
                    Span::styled(" yes   any other key: no", theme::dim()),
                ]))
            }
        }
    }
}

fn default_path(format: Format) -> String {
    let day = Local::now().format("%Y-%m-%d");
    format!("stealthlingo-{day}.{}", format.extension())
}

impl View for Transfer {
    fn title(&self) -> String {
        match self.kind {
            Kind::Export { .. } => "Export".to_string(),
            Kind::Import => "Import".to_string(),
        }
    }

    fn hints(&self) -> Vec<(&'static str, &'static str)> {
        match self.phase {
            Phase::Confirm => vec![("y/l", "confirm"), ("any key", "cancel")],
            Phase::Edit => match self.kind {
                Kind::Export { .. } => {
                    vec![("Tab", "format"), ("Enter", "export"), ("^C", "back")]
                }
                Kind::Import => vec![("Enter", "import"), ("^C", "back")],
            },
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let [body, prompt] =
            Layout::vertical([Constraint::Min(3), Constraint::Length(2)]).areas(area);

        frame.render_widget(
            Paragraph::new(vec![
                Line::from(vec![
                    Span::styled("Format  ", theme::dim()),
                    Span::styled(self.format_label(), theme::strong()),
                ]),
                Line::from(vec![Span::styled("Path    ", theme::dim())]),
                Line::from(vec![
                    Span::styled("Relative to ", theme::dim()),
                    Span::styled(self.cwd.clone(), theme::dim()),
                ]),
            ]),
            body,
        );

        let path_row = Rect {
            x: body.x + 8,
            y: body.y + 1,
            width: body.width.saturating_sub(8),
            height: 1,
        };
        if path_row.width > 0 {
            self.path
                .render(frame, path_row, Span::styled("› ", theme::heading()));
        }

        if let Some(line) = self.confirm_prompt() {
            frame.render_widget(Paragraph::new(line), prompt);
        }
    }

    fn handle_key(&mut self, ctx: &mut Context, key: KeyEvent, fx: &mut Fx) -> Result<()> {
        if matches!(self.phase, Phase::Confirm) {
            if matches!(
                key.code,
                KeyCode::Char('y') | KeyCode::Char('l') | KeyCode::Char('L')
            ) {
                match self.kind {
                    Kind::Export { .. } => self.run_export(ctx, fx),
                    Kind::Import => self.run_import(ctx, fx),
                }
            } else {
                self.phase = Phase::Edit;
            }
            return Ok(());
        }

        // Letters and arrows belong to the path; Ctrl+C (`interrupt`) goes back.
        match key.code {
            KeyCode::Tab if matches!(self.kind, Kind::Export { .. }) => {
                self.toggle_format();
                return Ok(());
            }
            KeyCode::Enter => {
                let file = self.path_buf();
                if file.as_os_str().is_empty() {
                    fx.flash(Tone::Bad, "Enter a file path.");
                    return Ok(());
                }
                match self.kind {
                    Kind::Export { .. } => {
                        if file.exists() {
                            self.phase = Phase::Confirm;
                        } else {
                            self.run_export(ctx, fx);
                        }
                    }
                    Kind::Import => {
                        if Format::from_path(&file).is_none() {
                            fx.flash(
                                Tone::Bad,
                                "unsupported file type; use a .json (full backup) or .csv (word list) path",
                            );
                        } else if !file.is_file() {
                            fx.flash(
                                Tone::Bad,
                                format!(
                                    "File not found: {}. Relative paths are under {}.",
                                    file.display(),
                                    self.cwd
                                ),
                            );
                        } else {
                            self.phase = Phase::Confirm;
                        }
                    }
                }
                return Ok(());
            }
            _ => {
                self.path.handle(key);
            }
        }
        Ok(())
    }

    fn interrupt(&mut self, _ctx: &mut Context, fx: &mut Fx) -> Result<bool> {
        match self.phase {
            Phase::Confirm => {
                self.phase = Phase::Edit;
                Ok(true)
            }
            Phase::Edit => {
                fx.go(Target::Settings);
                Ok(true)
            }
        }
    }
}
