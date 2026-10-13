//! Full-screen interface: a home screen with today's numbers, study sessions
//! with single-key answers, the word list, lookups and stats. Esc hides the
//! interface and leaves an interactive shell; Esc brings the interface back.

mod audio;
mod home;
mod lookup;
mod settings;
mod shell;
mod stats;
mod study;
mod theme;
mod transfer;
mod widgets;
mod words;

use std::io::{self, IsTerminal};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use ratatui::crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::crossterm::{cursor, execute};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::{DefaultTerminal, Frame};
use semver::Version;

use crate::commands::study::{prepare, Prepared, SessionRequest};
use crate::commands::Context;
use crate::dictionary::{normalize_headword, Entry};
use crate::storage::CachedWord;
use audio::AudioWorker;

/// The first screen to show.
pub enum Start {
    Home,
    Session(SessionRequest),
}

/// Whether both input and output are an interactive terminal.
pub fn available() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

pub use shell::inside_esc_shell;

#[derive(Debug, Clone)]
enum Target {
    Home,
    Session(SessionRequest),
    Words,
    Lookup,
    /// A cached entry opened from the word list.
    Entry(Box<CachedWord>),
    Stats,
    Settings,
    Transfer(transfer::Start),
}

/// Narrower headers leave out the new-release notice so the title still fits.
const UPDATE_MIN_WIDTH: u16 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tone {
    Info,
    Good,
    Bad,
}

/// A one-line message shown in the footer until the next key press.
#[derive(Debug, Clone)]
struct Flash {
    text: String,
    tone: Tone,
}

enum Effect {
    Go(Target),
    Quit,
    Play {
        headword: String,
        url: String,
        accent: Option<String>,
    },
    Flash(Flash),
}

/// Requests a screen makes of the app while handling a key.
#[derive(Default)]
struct Fx(Vec<Effect>);

impl Fx {
    fn go(&mut self, target: Target) {
        self.0.push(Effect::Go(target));
    }

    fn quit(&mut self) {
        self.0.push(Effect::Quit);
    }

    /// Plays the recording in `accent`, or the default one if there is none.
    fn play(&mut self, entry: &Entry, accent: Option<&str>) {
        let recording = entry.audio_in(accent);
        match recording.and_then(|p| p.audio_url.as_deref()) {
            Some(url) => self.0.push(Effect::Play {
                headword: normalize_headword(&entry.word),
                url: url.to_string(),
                accent: recording.and_then(|p| p.accent.clone()),
            }),
            None => self.flash(Tone::Info, "No pronunciation audio for this word."),
        }
    }

    fn flash(&mut self, tone: Tone, text: impl Into<String>) {
        let text: String = text.into();
        self.0.push(Effect::Flash(Flash {
            text: text.split_whitespace().collect::<Vec<_>>().join(" "),
            tone,
        }));
    }
}

/// Which recording the audio key plays for the entry on screen: the default
/// accent first, then each other recorded accent in turn.
#[derive(Debug, Clone, Default)]
struct Voice {
    last: Option<String>,
}

impl Voice {
    fn next<'e>(&self, entry: &'e Entry, preferred: &str) -> Option<&'e str> {
        let accents = entry.recording_accents();
        let start = match &self.last {
            Some(last) => accents
                .iter()
                .position(|a| a == last)
                .map_or(0, |i| (i + 1) % accents.len()),
            None => accents.iter().position(|a| *a == preferred).unwrap_or(0),
        };
        accents.get(start).copied()
    }

    /// Plays the next accent.
    fn play(&mut self, entry: &Entry, preferred: &str, fx: &mut Fx) {
        let accent = self.next(entry, preferred).map(str::to_string);
        fx.play(entry, accent.as_deref());
        self.last = accent;
    }

    /// Plays the accent heard last again.
    fn replay(&self, entry: &Entry, preferred: &str, fx: &mut Fx) {
        fx.play(entry, Some(self.last.as_deref().unwrap_or(preferred)));
    }

    /// Hint for the audio key: names the accent when the word has several, or
    /// when its only recording is not in the preferred accent.
    fn hint(&self, entry: &Entry, preferred: &str) -> &'static str {
        match entry.recording_accents().as_slice() {
            [only] if *only != preferred => {
                return match *only {
                    "UK" => "UK only",
                    "US" => "US only",
                    _ => "audio",
                };
            }
            [] | [_] => return "audio",
            _ => {}
        }
        match self.next(entry, preferred) {
            Some("UK") => "audio UK",
            Some("US") => "audio US",
            _ => "other accent",
        }
    }
}

trait View {
    fn title(&self) -> String;
    fn hints(&self) -> Vec<(&'static str, &'static str)>;
    fn render(&mut self, frame: &mut Frame, area: Rect);
    fn handle_key(&mut self, ctx: &mut Context, key: KeyEvent, fx: &mut Fx) -> Result<()>;

    /// Ctrl+C. Screens go back only while text is being typed; returning
    /// false quits the app.
    fn interrupt(&mut self, _ctx: &mut Context, _fx: &mut Fx) -> Result<bool> {
        Ok(false)
    }

    /// Slow work (such as a dictionary request) to run after the next draw,
    /// so a "loading" state is visible meanwhile.
    fn has_background_work(&self) -> bool {
        false
    }

    fn background_work(&mut self, _ctx: &mut Context, _fx: &mut Fx) -> Result<()> {
        Ok(())
    }

    fn set_paused(&mut self, _paused: bool) {}
}

pub struct App {
    screen: Box<dyn View>,
    flash: Option<Flash>,
    audio: Option<AudioWorker>,
    hidden: bool,
    quit: bool,
    /// A newer release, shown in the header.
    update: Option<String>,
    update_check: Option<Receiver<Version>>,
}

impl App {
    pub fn new(ctx: &mut Context, start: Start) -> Result<Self> {
        let mut app = Self {
            screen: Box::new(home::Home::load(ctx)?),
            flash: None,
            audio: None,
            hidden: false,
            quit: false,
            update: ctx
                .config
                .check_updates
                .then(|| crate::update::known_newer(&ctx.paths))
                .flatten()
                .map(|v| v.to_string()),
            update_check: None,
        };
        if !ctx.config_warnings.is_empty() {
            app.flash = Some(Flash {
                text: format!("config.json: {}", ctx.config_warnings.join("; ")),
                tone: Tone::Info,
            });
        }
        if let Start::Session(request) = start {
            let mut fx = Fx::default();
            app.open_session(ctx, request, &mut fx)?;
            app.apply(ctx, fx);
        }
        Ok(app)
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    pub fn is_hidden(&self) -> bool {
        self.hidden
    }

    /// Hiding pauses the session clock.
    pub fn set_hidden(&mut self, hidden: bool) {
        self.hidden = hidden;
        self.screen.set_paused(hidden);
    }

    fn notify(&mut self, tone: Tone, text: impl Into<String>) {
        let text: String = text.into();
        self.flash = Some(Flash {
            text: text.split_whitespace().collect::<Vec<_>>().join(" "),
            tone,
        });
    }

    /// The footer message, if any.
    pub fn flash_text(&self) -> Option<&str> {
        self.flash.as_ref().map(|f| f.text.as_str())
    }

    pub fn handle_key(&mut self, ctx: &mut Context, key: KeyEvent) {
        self.flash = None;
        let mut fx = Fx::default();
        let result = if widgets::is_ctrl(&key, 'c') {
            match self.screen.interrupt(ctx, &mut fx) {
                Ok(true) => Ok(()),
                Ok(false) => {
                    fx.quit();
                    Ok(())
                }
                Err(err) => Err(err),
            }
        } else {
            self.screen.handle_key(ctx, key, &mut fx)
        };
        if let Err(err) = result {
            fx.flash(Tone::Bad, format!("{err:#}"));
        }
        self.apply(ctx, fx);
    }

    pub fn has_background_work(&self) -> bool {
        self.screen.has_background_work()
    }

    pub fn background_work(&mut self, ctx: &mut Context) {
        let mut fx = Fx::default();
        if let Err(err) = self.screen.background_work(ctx, &mut fx) {
            fx.flash(Tone::Bad, format!("{err:#}"));
        }
        self.apply(ctx, fx);
    }

    /// Shows the release `check` finds, if it is newer.
    pub fn watch_update(&mut self, check: Receiver<Version>) {
        self.update_check = Some(check);
    }

    /// Picks up finished audio playback and the update check.
    pub fn tick(&mut self) {
        if let Some(err) = self.audio.as_mut().and_then(AudioWorker::poll) {
            self.flash = Some(Flash {
                text: format!("Could not play audio: {err}"),
                tone: Tone::Bad,
            });
        }
        if let Some(check) = &self.update_check {
            match check.try_recv() {
                Ok(version) => {
                    self.update = Some(version.to_string());
                    self.update_check = None;
                }
                Err(TryRecvError::Disconnected) => self.update_check = None,
                Err(TryRecvError::Empty) => {}
            }
        }
    }

    fn apply(&mut self, ctx: &mut Context, mut fx: Fx) {
        while !fx.0.is_empty() {
            for effect in std::mem::take(&mut fx.0) {
                match effect {
                    Effect::Go(target) => {
                        if let Err(err) = self.open(ctx, target, &mut fx) {
                            fx.flash(Tone::Bad, format!("{err:#}"));
                        }
                    }
                    Effect::Quit => self.quit = true,
                    Effect::Play {
                        headword,
                        url,
                        accent,
                    } => {
                        if self.audio.is_none() {
                            match ctx.audio_player() {
                                Ok(player) => self.audio = Some(AudioWorker::new(player)),
                                Err(err) => fx.flash(Tone::Bad, format!("{err:#}")),
                            }
                        }
                        if let Some(audio) = &mut self.audio {
                            audio.play(headword, url, accent);
                        }
                    }
                    Effect::Flash(flash) => self.flash = Some(flash),
                }
            }
        }
    }

    fn open(&mut self, ctx: &mut Context, target: Target, fx: &mut Fx) -> Result<()> {
        if matches!(
            target,
            Target::Home | Target::Stats | Target::Settings | Target::Session(_)
        ) {
            match ctx.reload_config() {
                Ok(warnings) if !warnings.is_empty() => {
                    fx.flash(Tone::Info, format!("config.json: {}", warnings.join("; ")));
                }
                Ok(_) => {}
                Err(err) => fx.flash(Tone::Bad, format!("{err:#}; keeping the current settings")),
            }
        }
        self.screen = match target {
            Target::Home => Box::new(home::Home::load(ctx)?),
            Target::Session(request) => return self.open_session(ctx, request, fx),
            Target::Words => Box::new(words::Words::load(ctx)?),
            Target::Lookup => Box::new(lookup::Lookup::new(Target::Home, &ctx.config.accent)),
            Target::Entry(cached) => Box::new(lookup::Lookup::with_entry(
                *cached,
                Target::Words,
                &ctx.config.accent,
            )),
            Target::Stats => Box::new(stats::Stats::load(ctx)?),
            Target::Settings => Box::new(settings::Settings::load(ctx)?),
            Target::Transfer(start) => Box::new(transfer::Transfer::new(start)),
        };
        Ok(())
    }

    /// Starts a session, or stays on the current screen and explains why
    /// there is nothing to practice.
    fn open_session(
        &mut self,
        ctx: &mut Context,
        request: SessionRequest,
        fx: &mut Fx,
    ) -> Result<()> {
        match prepare(ctx, &request)? {
            Prepared::Ready { plan, queue, .. } => {
                self.screen = Box::new(study::Study::start(ctx, request, plan, queue, fx));
            }
            Prepared::Empty(lines) => fx.flash(Tone::Info, lines.join(" ")),
        }
        Ok(())
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        if area.width < 40 || area.height < 12 {
            frame.render_widget(
                Paragraph::new("Terminal too small. Enlarge it, or press Esc to hide.")
                    .wrap(ratatui::widgets::Wrap { trim: true }),
                area,
            );
            return;
        }
        let [header, _, body, footer] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(area);

        let left = Line::from(vec![
            Span::styled(" StealthLingo", theme::strong()),
            Span::styled("  ·  ", theme::dim()),
            Span::styled(self.screen.title(), theme::accent()),
        ]);
        let playing = match self.audio.as_ref().and_then(AudioWorker::playing) {
            Some(Some(accent)) => format!("♪ playing {accent}   "),
            Some(None) => "♪ playing   ".to_string(),
            None => String::new(),
        };
        let update = match &self.update {
            Some(version) if area.width >= UPDATE_MIN_WIDTH => format!("update {version}   "),
            _ => String::new(),
        };
        let right = Line::from(vec![
            Span::styled(update, theme::dim()),
            Span::styled(playing, theme::accent()),
            Span::styled("Esc", theme::heading()),
            Span::styled(" hide ", theme::dim()),
        ])
        .right_aligned();
        frame.render_widget(Paragraph::new(left), header);
        frame.render_widget(Paragraph::new(right), header);

        let body = widgets::centered(body, 96).inner(ratatui::layout::Margin::new(2, 0));
        self.screen.render(frame, body);

        let footer_line = match &self.flash {
            Some(flash) => {
                let style = match flash.tone {
                    Tone::Info => theme::warn(),
                    Tone::Good => theme::good(),
                    Tone::Bad => theme::bad(),
                };
                Line::from(vec![
                    Span::raw(" "),
                    Span::styled(flash.text.clone(), style),
                ])
            }
            None => {
                let mut line = theme::hints(&self.screen.hints());
                line.spans.insert(0, Span::raw(" "));
                line
            }
        };
        frame.render_widget(Paragraph::new(footer_line), footer);
    }
}

static RUNNING: AtomicBool = AtomicBool::new(false);
static SIGNALLED: AtomicBool = AtomicBool::new(false);

/// Called by the Ctrl+C handler; returns false when the full-screen interface
/// is not running. The first signal asks the event loop to quit so the
/// terminal is restored; a second one, while the loop is still blocked (e.g.
/// on a slow lookup), restores the terminal here and exits.
pub fn on_interrupt_signal() -> bool {
    if !RUNNING.load(Ordering::SeqCst) {
        return false;
    }
    if SIGNALLED.swap(true, Ordering::SeqCst) {
        restore_terminal();
        std::process::exit(130);
    }
    true
}

fn restore_terminal() {
    let _ = execute!(io::stdout(), cursor::Show);
    ratatui::restore();
}

/// Leaves the full-screen interface even when the app fails or panics.
struct Restore;

impl Restore {
    fn new() -> Self {
        SIGNALLED.store(false, Ordering::SeqCst);
        RUNNING.store(true, Ordering::SeqCst);
        Self
    }
}

impl Drop for Restore {
    fn drop(&mut self) {
        restore_terminal();
        RUNNING.store(false, Ordering::SeqCst);
    }
}

pub fn run(ctx: &mut Context, start: Start) -> Result<()> {
    // Study and review never touch the network; only the home screen checks.
    let at_home = matches!(start, Start::Home);
    let mut app = App::new(ctx, start)?;
    if at_home {
        if let Some(check) = crate::commands::update::start_daily_check(ctx) {
            app.watch_update(check);
        }
    }
    let origin = shell::main_screen_origin();
    let mut terminal = ratatui::try_init()?;
    let _restore = Restore::new();
    event_loop(&mut terminal, &mut app, ctx, origin)
}

/// How long the interface is on screen before the Esc shell starts in the
/// background, so starting a shell does not compete with opening the app.
const SHELL_PRELOAD_DELAY: Duration = Duration::from_secs(1);

fn event_loop(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    ctx: &mut Context,
    origin: Option<u16>,
) -> Result<()> {
    let mut slot = shell::ShellSlot::new();
    slot.set_origin(origin);
    let opened = Instant::now();
    while !app.should_quit() && !SIGNALLED.load(Ordering::SeqCst) {
        match slot.poll() {
            Some(Ok(())) => {
                let (cols, rows) = term_size(terminal);
                slot.resize(cols, rows);
                if app.is_hidden() {
                    slot.show(true);
                }
            }
            // A background start that fails stays quiet until Esc tries again.
            Some(Err(err)) => {
                if app.is_hidden() {
                    show_app(terminal, app, &mut slot)?;
                    app.notify(Tone::Bad, format!("{err:#}"));
                }
            }
            None => {}
        }
        if slot.exited() {
            if app.is_hidden() {
                // `exit` in the shell ends it. Come back rather than sit on a dead prompt.
                show_app(terminal, app, &mut slot)?;
                slot.clear();
            } else {
                slot.discard();
            }
        }
        if !app.is_hidden() {
            terminal.draw(|frame| app.render(frame))?;
            if app.has_background_work() {
                app.background_work(ctx);
                continue;
            }
            if opened.elapsed() >= SHELL_PRELOAD_DELAY {
                let (cols, rows) = term_size(terminal);
                slot.preload(cols, rows);
            }
        }
        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) => on_key(terminal, app, ctx, &mut slot, key)?,
                Event::Resize(cols, rows) => slot.resize(cols, rows),
                _ => {}
            }
        }
        app.tick();
    }
    Ok(())
}

fn term_size(terminal: &DefaultTerminal) -> (u16, u16) {
    terminal
        .size()
        .map(|size| (size.width.max(1), size.height.max(1)))
        .unwrap_or((80, 24))
}

/// Leaves the interface on the main screen and shows the shell. The shell is
/// normally already running in the background; if it is still starting, the
/// screen changes anyway and the prompt appears once it is ready.
fn hide_app(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    slot: &mut shell::ShellSlot,
) -> Result<()> {
    execute!(io::stdout(), LeaveAlternateScreen, cursor::Show)?;
    app.set_hidden(true);
    if slot.exited() {
        slot.clear();
    }
    let (cols, rows) = term_size(terminal);
    if slot.is_ready() {
        slot.resize(cols, rows);
        slot.show(true);
        return Ok(());
    }
    if !slot.is_starting() {
        if let Err(err) = slot.start(cols, rows) {
            show_app(terminal, app, slot)?;
            app.notify(Tone::Bad, format!("Could not start a shell: {err}"));
        }
    }
    Ok(())
}

fn show_app(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    slot: &mut shell::ShellSlot,
) -> Result<()> {
    // Stop shell output before the alternate screen, so it is not drawn over the interface.
    slot.show(false);
    slot.set_origin(shell::main_screen_origin());
    execute!(io::stdout(), EnterAlternateScreen, cursor::Hide)?;
    terminal.clear()?;
    app.set_hidden(false);
    Ok(())
}

fn on_key(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    ctx: &mut Context,
    slot: &mut shell::ShellSlot,
    key: KeyEvent,
) -> Result<()> {
    if key.kind == KeyEventKind::Release {
        return Ok(());
    }
    if shell::boss_key(&key) {
        if app.is_hidden() {
            show_app(terminal, app, slot)?;
        } else {
            hide_app(terminal, app, slot)?;
        }
        return Ok(());
    }
    if app.is_hidden() {
        if let Some(bytes) = shell::shell_bytes(&key) {
            if slot.send(&bytes).is_err() {
                show_app(terminal, app, slot)?;
                slot.clear();
            }
        }
        return Ok(());
    }
    app.handle_key(ctx, key);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::Phonetic;

    fn recording(accent: &str) -> Phonetic {
        Phonetic {
            text: None,
            audio_url: Some(format!("https://x/{accent}.ogg")),
            accent: Some(accent.to_string()),
        }
    }

    fn played(fx: &Fx) -> Option<String> {
        match fx.0.last() {
            Some(Effect::Play { accent, .. }) => accent.clone(),
            _ => None,
        }
    }

    #[test]
    fn interrupt_signal_asks_the_running_interface_to_quit() {
        assert!(!on_interrupt_signal());
        RUNNING.store(true, Ordering::SeqCst);
        assert!(on_interrupt_signal());
        assert!(SIGNALLED.swap(false, Ordering::SeqCst));
        RUNNING.store(false, Ordering::SeqCst);
    }

    #[test]
    fn audio_key_alternates_between_accents() {
        let entry = Entry {
            word: "tomato".to_string(),
            phonetics: vec![recording("US"), recording("UK")],
            ..Entry::default()
        };
        let mut voice = Voice::default();
        let mut fx = Fx::default();
        assert_eq!(voice.hint(&entry, "US"), "audio US");
        voice.play(&entry, "US", &mut fx);
        assert_eq!(played(&fx).as_deref(), Some("US"));
        assert_eq!(voice.hint(&entry, "US"), "audio UK");
        voice.play(&entry, "US", &mut fx);
        assert_eq!(played(&fx).as_deref(), Some("UK"));
        voice.replay(&entry, "US", &mut fx);
        assert_eq!(played(&fx).as_deref(), Some("UK"));
        voice.play(&entry, "US", &mut fx);
        assert_eq!(played(&fx).as_deref(), Some("US"));
    }

    #[test]
    fn single_recordings_play_whatever_accent_they_have() {
        let entry = Entry {
            word: "hostel".to_string(),
            phonetics: vec![recording("US")],
            ..Entry::default()
        };
        let mut voice = Voice::default();
        let mut fx = Fx::default();
        assert_eq!(voice.hint(&entry, "UK"), "US only");
        assert_eq!(voice.hint(&entry, "US"), "audio");
        voice.play(&entry, "UK", &mut fx);
        voice.play(&entry, "UK", &mut fx);
        assert_eq!(played(&fx).as_deref(), Some("US"));
    }
}
