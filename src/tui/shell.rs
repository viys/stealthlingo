//! The shell shown while the full-screen interface is hidden.
//!
//! The shell that launched StealthLingo is blocked waiting for this process, so
//! it cannot read the keyboard. Hiding therefore starts a new interactive shell
//! on a pseudoterminal. Every key is forwarded except a lone Esc, which is the
//! boss key and is never written to that shell.

use std::env;
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::mpsc::{self, TryRecvError};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use unicode_width::UnicodeWidthChar;

/// Output kept while the interface is visible. Past this, the reader waits and
/// the child blocks instead of the buffer growing without limit.
const MAX_PENDING: usize = 1024 * 1024;

/// A lone Esc press returns to the interface. Repeats are ignored so holding
/// the key does not toggle on every repeat, and Esc is never part of a chord
/// the shell should see (Ctrl+[ arrives as Esc).
pub(super) fn boss_key(key: &KeyEvent) -> bool {
    key.kind == KeyEventKind::Press
        && key.code == KeyCode::Esc
        && !key.modifiers.contains(KeyModifiers::CONTROL)
        && !key.modifiers.contains(KeyModifiers::ALT)
}

/// Bytes to write to the shell for this key, in the sequences a terminal would
/// send. Esc produces nothing: the caller handles it.
pub(super) fn shell_bytes(key: &KeyEvent) -> Option<Vec<u8>> {
    let modifiers = key.modifiers;
    match key.code {
        KeyCode::Char(ch) if modifiers.contains(KeyModifiers::CONTROL) => {
            let byte = ctrl_byte(ch)?;
            let mut bytes = vec![byte];
            if modifiers.contains(KeyModifiers::ALT) {
                bytes.insert(0, 0x1b);
            }
            Some(bytes)
        }
        KeyCode::Char(ch) => {
            let mut bytes = ch.to_string().into_bytes();
            if modifiers.contains(KeyModifiers::ALT) {
                bytes.insert(0, 0x1b);
            }
            Some(bytes)
        }
        KeyCode::Enter => Some(prefixed(modifiers, vec![b'\r'])),
        KeyCode::Backspace => Some(prefixed(modifiers, vec![0x7f])),
        KeyCode::Tab if !modifiers.contains(KeyModifiers::CONTROL) => {
            Some(prefixed(modifiers, vec![b'\t']))
        }
        KeyCode::BackTab => Some(csi("", modifier_param(modifiers), b'Z')),
        KeyCode::Up => Some(csi("", modifier_param(modifiers), b'A')),
        KeyCode::Down => Some(csi("", modifier_param(modifiers), b'B')),
        KeyCode::Right => Some(csi("", modifier_param(modifiers), b'C')),
        KeyCode::Left => Some(csi("", modifier_param(modifiers), b'D')),
        KeyCode::Home => Some(csi("", modifier_param(modifiers), b'H')),
        KeyCode::End => Some(csi("", modifier_param(modifiers), b'F')),
        KeyCode::Insert => Some(csi("2", modifier_param(modifiers), b'~')),
        KeyCode::Delete => Some(csi("3", modifier_param(modifiers), b'~')),
        KeyCode::PageUp => Some(csi("5", modifier_param(modifiers), b'~')),
        KeyCode::PageDown => Some(csi("6", modifier_param(modifiers), b'~')),
        KeyCode::F(n) => f_key(n).map(|param| csi(param, modifier_param(modifiers), b'~')),
        KeyCode::Null => Some(vec![0]),
        KeyCode::Esc
        | KeyCode::Tab
        | KeyCode::CapsLock
        | KeyCode::ScrollLock
        | KeyCode::NumLock
        | KeyCode::PrintScreen
        | KeyCode::Pause
        | KeyCode::Menu
        | KeyCode::KeypadBegin
        | KeyCode::Media(_)
        | KeyCode::Modifier(_) => None,
    }
}

/// Alt prefixes a simple key with Esc. Control and shift for these keys are
/// either in the byte itself or ignored, matching a classic terminal.
fn prefixed(modifiers: KeyModifiers, mut bytes: Vec<u8>) -> Vec<u8> {
    if modifiers.contains(KeyModifiers::ALT) {
        bytes.insert(0, 0x1b);
    }
    bytes
}

fn ctrl_byte(ch: char) -> Option<u8> {
    match ch.to_ascii_lowercase() {
        'a'..='z' => Some(ch.to_ascii_lowercase() as u8 - b'a' + 1),
        ' ' | '@' => Some(0),
        '\\' => Some(0x1c),
        ']' => Some(0x1d),
        '^' | '~' => Some(0x1e),
        '_' => Some(0x1f),
        '?' => Some(0x7f),
        _ => None,
    }
}

/// xterm modifier parameter: 1 + shift + 2×alt + 4×ctrl.
fn modifier_param(modifiers: KeyModifiers) -> u8 {
    let mut param = 1u8;
    if modifiers.contains(KeyModifiers::SHIFT) {
        param += 1;
    }
    if modifiers.contains(KeyModifiers::ALT) {
        param += 2;
    }
    if modifiers.contains(KeyModifiers::CONTROL) {
        param += 4;
    }
    param
}

fn csi(param: &str, modifier: u8, final_byte: u8) -> Vec<u8> {
    let mut out = vec![0x1b, b'['];
    if modifier == 1 {
        out.extend_from_slice(param.as_bytes());
    } else if param.is_empty() {
        out.extend_from_slice(format!("1;{modifier}").as_bytes());
    } else {
        out.extend_from_slice(format!("{param};{modifier}").as_bytes());
    }
    out.push(final_byte);
    out
}

fn f_key(n: u8) -> Option<&'static str> {
    Some(match n {
        1 => "11",
        2 => "12",
        3 => "13",
        4 => "14",
        5 => "15",
        6 => "17",
        7 => "18",
        8 => "19",
        9 => "20",
        10 => "21",
        11 => "23",
        12 => "24",
        _ => return None,
    })
}

#[derive(Clone, Copy)]
enum Host {
    #[cfg(any(windows, test))]
    Windows,
    #[cfg(any(not(windows), test))]
    Unix,
}

struct ShellLaunch {
    program: String,
    args: Vec<String>,
}

/// Which program to start, and the arguments that keep it interactive.
/// `ancestor` is the nearest shell process above us (Windows). `$SHELL` is
/// the Unix choice; on Windows it is only a fallback, and an MSYS path
/// (`/usr/bin/bash`) is skipped because `CreateProcess` cannot run it.
fn shell_launch(host: Host, shell_env: Option<&str>, ancestor: Option<&str>) -> ShellLaunch {
    let program = match host {
        #[cfg(any(windows, test))]
        Host::Windows => ancestor
            .filter(|program| !program.is_empty())
            .map(str::to_string)
            .or_else(|| usable_shell_env(host, shell_env))
            .unwrap_or_else(|| "powershell.exe".to_string()),
        #[cfg(any(not(windows), test))]
        Host::Unix => usable_shell_env(host, shell_env).unwrap_or_else(|| "/bin/sh".to_string()),
    };
    let args = if no_logo(&program) {
        vec!["-NoLogo".to_string()]
    } else {
        Vec::new()
    };
    ShellLaunch { program, args }
}

fn usable_shell_env(host: Host, shell_env: Option<&str>) -> Option<String> {
    let value = shell_env?.trim();
    if value.is_empty() || (host_is_windows(host) && value.starts_with('/')) {
        return None;
    }
    Some(value.to_string())
}

fn host_is_windows(host: Host) -> bool {
    match host {
        #[cfg(any(windows, test))]
        Host::Windows => true,
        #[cfg(any(not(windows), test))]
        Host::Unix => false,
    }
}

fn no_logo(program: &str) -> bool {
    matches!(
        file_name(program).to_ascii_lowercase().as_str(),
        "powershell.exe" | "powershell" | "pwsh.exe" | "pwsh"
    )
}

fn file_name(path: &str) -> &str {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
}

fn user_shell() -> ShellLaunch {
    let shell_env = env::var("SHELL").ok();
    #[cfg(windows)]
    {
        shell_launch(
            Host::Windows,
            shell_env.as_deref(),
            win32::shell_ancestor().as_deref(),
        )
    }
    #[cfg(not(windows))]
    {
        shell_launch(Host::Unix, shell_env.as_deref(), None)
    }
}

struct GateInner {
    show: bool,
    stop: bool,
    pending: Vec<u8>,
}

struct Gate {
    inner: Mutex<GateInner>,
    ready: Condvar,
}

impl Gate {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(GateInner {
                show: false,
                stop: false,
                pending: Vec::new(),
            }),
            ready: Condvar::new(),
        })
    }

    fn set_show(&self, show: bool) {
        let mut guard = lock(&self.inner);
        guard.show = show;
        if show && !guard.pending.is_empty() {
            let mut out = io::stdout();
            let _ = out.write_all(&guard.pending);
            let _ = out.flush();
            guard.pending.clear();
        }
        self.ready.notify_all();
    }

    fn stop(&self) {
        let mut guard = lock(&self.inner);
        guard.stop = true;
        guard.show = false;
        self.ready.notify_all();
    }

    fn push(&self, chunk: &[u8]) -> bool {
        let mut guard = lock(&self.inner);
        loop {
            if guard.stop {
                return false;
            }
            if guard.show {
                let mut out = io::stdout();
                let _ = out.write_all(&guard.pending);
                let _ = out.write_all(chunk);
                let _ = out.flush();
                guard.pending.clear();
                return true;
            }
            if guard.pending.len() < MAX_PENDING {
                guard.pending.extend_from_slice(chunk);
                return true;
            }
            guard = wait(&self.ready, guard);
        }
    }

    #[cfg(test)]
    fn pending(&self) -> Vec<u8> {
        lock(&self.inner).pending.clone()
    }
}

fn lock(mutex: &Mutex<GateInner>) -> MutexGuard<'_, GateInner> {
    mutex.lock().unwrap_or_else(|err| err.into_inner())
}

fn wait<'a>(ready: &'a Condvar, guard: MutexGuard<'a, GateInner>) -> MutexGuard<'a, GateInner> {
    ready.wait(guard).unwrap_or_else(|err| err.into_inner())
}

/// Cursor tracking for the replies a terminal has to write back.
///
/// ConPTY asks for the cursor position (`ESC [ 6 n`) while a console program
/// is starting, and waits until the terminal answers. Leaving that unanswered
/// stalls the shell before it prints a prompt.
struct TermReply {
    row: u16,
    col: u16,
    held: Vec<u8>,
}

impl TermReply {
    fn new() -> Self {
        Self {
            row: 1,
            col: 1,
            held: Vec::new(),
        }
    }

    /// Bytes to show, and bytes to write back to the shell.
    fn push(&mut self, input: &[u8], cols: u16, rows: u16) -> (Vec<u8>, Vec<u8>) {
        let cols = cols.max(1);
        let rows = rows.max(1);
        self.held.extend_from_slice(input);
        let mut display = Vec::new();
        let mut reply = Vec::new();
        loop {
            if self.held.is_empty() {
                break;
            }
            if self.held[0] != 0x1b {
                let len = text_len(&self.held);
                if len == 0 {
                    break;
                }
                let text: Vec<u8> = self.held[..len].to_vec();
                self.advance(&text, cols, rows);
                display.extend(self.held.drain(..len));
                continue;
            }
            match classify_escape(&self.held) {
                Escape::NeedMore => break,
                Escape::Show(len) => display.extend(self.held.drain(..len)),
                Escape::Answer(len, kind) => {
                    self.held.drain(..len);
                    reply.extend_from_slice(kind.bytes());
                }
                Escape::Cursor(len) => {
                    self.held.drain(..len);
                    reply.extend(self.answer_cursor());
                }
                Escape::Move { len, row, col } => {
                    self.row = row.clamp(1, rows);
                    self.col = col.clamp(1, cols.saturating_add(1));
                    display.extend(self.held.drain(..len));
                }
            }
        }
        if self.held.len() > 128 {
            display.append(&mut self.held);
        }
        (display, reply)
    }

    fn advance(&mut self, bytes: &[u8], cols: u16, rows: u16) {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return;
        };
        for ch in text.chars() {
            match ch {
                '\r' => self.col = 1,
                '\n' => self.row = (self.row + 1).min(rows),
                '\u{8}' => self.col = self.col.saturating_sub(1).max(1),
                ch if (ch as u32) < 0x20 || ch == '\u{7f}' => {}
                ch => {
                    let width = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
                    if width == 0 {
                        continue;
                    }
                    if self.col.saturating_add(width) > cols.saturating_add(1) {
                        self.col = 1;
                        self.row = (self.row + 1).min(rows);
                    }
                    self.col = self.col.saturating_add(width);
                }
            }
        }
    }

    fn answer_cursor(&self) -> Vec<u8> {
        format!("\x1b[{};{}R", self.row, self.col).into_bytes()
    }
}

enum AnswerKind {
    Status,
    Attributes,
}

impl AnswerKind {
    fn bytes(self) -> &'static [u8] {
        match self {
            Self::Status => b"\x1b[0n",
            Self::Attributes => b"\x1b[?1;0c",
        }
    }
}

enum Escape {
    NeedMore,
    Show(usize),
    Answer(usize, AnswerKind),
    /// Cursor-position request. The answer depends on the tracked cursor.
    Cursor(usize),
    Move {
        len: usize,
        row: u16,
        col: u16,
    },
}

fn classify_escape(buf: &[u8]) -> Escape {
    if buf.len() < 2 {
        return Escape::NeedMore;
    }
    if buf[1] != b'[' {
        return Escape::Show(2);
    }
    let Some(final_at) = buf.iter().skip(2).position(|b| (0x40..=0x7e).contains(b)) else {
        return if buf.len() > 64 {
            Escape::Show(1)
        } else {
            Escape::NeedMore
        };
    };
    let final_at = final_at + 2;
    let len = final_at + 1;
    let params = &buf[2..final_at];
    let final_byte = buf[final_at];
    match final_byte {
        b'n' if params == b"6" => Escape::Cursor(len),
        b'n' if params == b"5" => Escape::Answer(len, AnswerKind::Status),
        b'c' if params.is_empty() || params == b"0" => Escape::Answer(len, AnswerKind::Attributes),
        b'H' | b'f' => {
            let (row, col) = cup_position(params);
            Escape::Move { len, row, col }
        }
        b'A' | b'B' | b'C' | b'D' => Escape::Show(len),
        _ => Escape::Show(len),
    }
}

fn cup_position(params: &[u8]) -> (u16, u16) {
    let mut parts = params.split(|b| *b == b';');
    let row = parts.next().map(number_or_one).unwrap_or(1);
    let col = parts.next().map(number_or_one).unwrap_or(1);
    (row, col)
}

fn number_or_one(bytes: &[u8]) -> u16 {
    if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
        return 1;
    }
    let mut n: u16 = 0;
    for &b in bytes {
        n = n.saturating_mul(10).saturating_add((b - b'0') as u16);
    }
    n.max(1)
}

/// Complete text before the next Esc. Zero means an incomplete UTF-8 sequence.
fn text_len(buf: &[u8]) -> usize {
    let end = buf.iter().position(|&b| b == 0x1b).unwrap_or(buf.len());
    match std::str::from_utf8(&buf[..end]) {
        Ok(_) => end,
        Err(err) if err.error_len().is_none() && end == buf.len() => err.valid_up_to(),
        Err(err) if err.valid_up_to() == 0 => 1,
        Err(err) => err.valid_up_to().max(1),
    }
}

struct Screen {
    cols: AtomicU16,
    rows: AtomicU16,
}

fn pump(
    mut reader: Box<dyn Read + Send>,
    gate: Arc<Gate>,
    alive: Arc<AtomicBool>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    screen: Arc<Screen>,
) {
    let mut term = TermReply::new();
    let mut buf = [0; 8192];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let (display, reply) = term.push(
                    &buf[..n],
                    screen.cols.load(Ordering::Relaxed),
                    screen.rows.load(Ordering::Relaxed),
                );
                if !reply.is_empty() {
                    let mut writer = writer.lock().unwrap_or_else(|err| err.into_inner());
                    let _ = writer.write_all(&reply);
                    let _ = writer.flush();
                }
                if !display.is_empty() && !gate.push(&display) {
                    break;
                }
            }
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    alive.store(false, Ordering::SeqCst);
}

struct ShellSession {
    // Dropped before the reader is joined: the reader blocks in `read` until
    // the pty is closed.
    master: Option<Box<dyn MasterPty + Send>>,
    writer: Option<Arc<Mutex<Box<dyn Write + Send>>>>,
    child: Box<dyn Child + Send + Sync>,
    #[cfg(unix)]
    pid: Option<u32>,
    #[cfg(windows)]
    job: Option<win32::OwnedHandle>,
    gate: Arc<Gate>,
    alive: Arc<AtomicBool>,
    screen: Arc<Screen>,
    reader: Option<JoinHandle<()>>,
}

impl ShellSession {
    fn open(program: &str, args: &[&str], cols: u16, rows: u16) -> Result<Self> {
        let pair = native_pty_system()
            .openpty(pty_size(cols, rows))
            .context("Could not open a pseudoterminal.")?;
        let mut cmd = CommandBuilder::new(program);
        for arg in args {
            cmd.arg(arg);
        }
        if let Ok(dir) = env::current_dir() {
            cmd.cwd(dir);
        }
        for (key, value) in env::vars_os() {
            if !key.is_empty() {
                cmd.env(key, value);
            }
        }
        if env::var_os("TERM").is_none() {
            cmd.env("TERM", "xterm-256color");
        }
        let mut child = pair
            .slave
            .spawn_command(cmd)
            .with_context(|| format!("Could not start {program}."))?;
        #[cfg(unix)]
        let pid = child.process_id();
        #[cfg(windows)]
        let job = child.process_id().and_then(win32::bind_kill_on_close);
        let reader = pair
            .master
            .try_clone_reader()
            .context("Could not read shell output.")?;
        let writer = Arc::new(Mutex::new(
            pair.master
                .take_writer()
                .context("Could not write to the shell.")?,
        ));
        let gate = Gate::new();
        let alive = Arc::new(AtomicBool::new(true));
        let screen = Arc::new(Screen {
            cols: AtomicU16::new(cols.max(1)),
            rows: AtomicU16::new(rows.max(1)),
        });
        let gate_for_reader = Arc::clone(&gate);
        let alive_for_reader = Arc::clone(&alive);
        let writer_for_reader = Arc::clone(&writer);
        let screen_for_reader = Arc::clone(&screen);
        let reader = match thread::Builder::new()
            .name("shell-output".into())
            .spawn(move || {
                pump(
                    reader,
                    gate_for_reader,
                    alive_for_reader,
                    writer_for_reader,
                    screen_for_reader,
                )
            }) {
            Ok(reader) => reader,
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(err).context("Could not read shell output.");
            }
        };
        Ok(Self {
            master: Some(pair.master),
            writer: Some(writer),
            child,
            #[cfg(unix)]
            pid,
            #[cfg(windows)]
            job,
            gate,
            alive,
            screen,
            reader: Some(reader),
        })
    }

    fn spawn_user(cols: u16, rows: u16) -> Result<Self> {
        let launch = user_shell();
        let args: Vec<&str> = launch.args.iter().map(String::as_str).collect();
        Self::open(&launch.program, &args, cols, rows)
    }

    fn set_show(&self, show: bool) {
        self.gate.set_show(show);
    }

    fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.screen.cols.store(cols.max(1), Ordering::Relaxed);
        self.screen.rows.store(rows.max(1), Ordering::Relaxed);
        if let Some(master) = &self.master {
            master.resize(pty_size(cols, rows))?;
        }
        Ok(())
    }

    fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        let Some(writer) = &self.writer else {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "shell is closed"));
        };
        let mut writer = writer.lock().unwrap_or_else(|err| err.into_inner());
        writer.write_all(bytes)?;
        writer.flush()
    }

    fn exited(&mut self) -> bool {
        if !self.alive.load(Ordering::SeqCst) {
            return true;
        }
        match self.child.try_wait() {
            Ok(Some(_)) => {
                self.alive.store(false, Ordering::SeqCst);
                true
            }
            Ok(None) => false,
            Err(_) => {
                self.alive.store(false, Ordering::SeqCst);
                true
            }
        }
    }

    fn kill_tree(&mut self) {
        #[cfg(windows)]
        {
            // Closing the job ends the shell and commands it started.
            self.job.take();
        }
        #[cfg(unix)]
        if let Some(pid) = self.pid.and_then(|pid| i32::try_from(pid).ok()) {
            if pid > 0 {
                // SAFETY: `kill` is the libc function; a negative pid addresses
                // the process group the shell leads, so a command it started
                // does not keep running after StealthLingo exits.
                unsafe { kill(-pid, 9) };
            }
        }
        let _ = self.child.kill();
    }
}

impl Drop for ShellSession {
    fn drop(&mut self) {
        self.gate.stop();
        self.kill_tree();
        // The reader is blocked in `read` until the pty closes.
        self.writer.take();
        self.master.take();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        let _ = self.child.wait();
    }
}

#[cfg(unix)]
unsafe extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

fn pty_size(cols: u16, rows: u16) -> PtySize {
    PtySize {
        rows: rows.max(1),
        cols: cols.max(1),
        pixel_width: 0,
        pixel_height: 0,
    }
}

struct PendingShell {
    rx: mpsc::Receiver<Result<ShellSession>>,
    thread: JoinHandle<()>,
}

impl PendingShell {
    fn spawn(cols: u16, rows: u16) -> io::Result<Self> {
        let (tx, rx) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("shell-spawn".into())
            .spawn(move || {
                let _ = tx.send(ShellSession::spawn_user(cols, rows));
            })?;
        Ok(Self { rx, thread })
    }

    /// Drops a shell that finished starting after the interface quit, and
    /// waits until that has happened so the process is not left behind.
    fn cancel(self) {
        drop(self.rx);
        let _ = self.thread.join();
    }
}

/// The shell for one full-screen session. It stays running across Esc toggles
/// so the next hide returns to the same prompt. `exit` in the shell ends it;
/// the next hide starts another.
pub(super) struct ShellSlot {
    session: Option<ShellSession>,
    pending: Option<PendingShell>,
}

impl ShellSlot {
    pub(super) fn new() -> Self {
        Self {
            session: None,
            pending: None,
        }
    }

    /// `Some(Ok(()))` when a shell has just become ready, `Some(Err)` when
    /// starting it failed, `None` when nothing changed.
    pub(super) fn poll(&mut self) -> Option<Result<()>> {
        let pending = self.pending.as_ref()?;
        let result = match pending.rx.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => {
                Err(anyhow::anyhow!("The shell stopped before it started."))
            }
        };
        self.pending.take();
        match result {
            Ok(session) => {
                self.session = Some(session);
                Some(Ok(()))
            }
            Err(err) => Some(Err(err)),
        }
    }

    pub(super) fn is_ready(&mut self) -> bool {
        match self.session.as_mut() {
            Some(session) => !session.exited(),
            None => false,
        }
    }

    pub(super) fn is_starting(&self) -> bool {
        self.pending.is_some()
    }

    /// True when a shell was running and has now exited.
    pub(super) fn exited(&mut self) -> bool {
        match self.session.as_mut() {
            Some(session) => session.exited(),
            None => false,
        }
    }

    pub(super) fn clear(&mut self) {
        self.session = None;
    }

    pub(super) fn start(&mut self, cols: u16, rows: u16) -> io::Result<()> {
        if self.session.is_some() || self.pending.is_some() {
            return Ok(());
        }
        self.pending = Some(PendingShell::spawn(cols, rows)?);
        Ok(())
    }

    pub(super) fn show(&self, show: bool) {
        if let Some(session) = &self.session {
            session.set_show(show);
        }
    }

    pub(super) fn resize(&self, cols: u16, rows: u16) {
        if let Some(session) = &self.session {
            let _ = session.resize(cols, rows);
        }
    }

    /// Forwards a key. Keys typed before the shell has started are ignored.
    /// A write error means the shell is gone.
    pub(super) fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        let Some(session) = self.session.as_mut() else {
            return Ok(());
        };
        session.send(bytes)
    }
}

impl Drop for ShellSlot {
    fn drop(&mut self) {
        if let Some(session) = &self.session {
            session.set_show(false);
        }
        self.session.take();
        if let Some(pending) = self.pending.take() {
            pending.cancel();
        }
    }
}

#[cfg(windows)]
mod win32 {
    use std::collections::HashMap;
    use std::ffi::c_void;

    use super::is_shell_name;

    const TH32CS_SNAPPROCESS: u32 = 0x0000_0002;
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const PROCESS_TERMINATE: u32 = 0x0001;
    const PROCESS_SET_QUOTA: u32 = 0x0100;
    const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x2000;
    const JOB_OBJECT_BASIC_LIMIT_INFORMATION: i32 = 2;
    const INVALID_HANDLE: *mut c_void = -1isize as *mut c_void;

    #[repr(C)]
    struct ProcessEntry32W {
        size: u32,
        usage: u32,
        pid: u32,
        default_heap_id: usize,
        module_id: u32,
        threads: u32,
        parent_pid: u32,
        pri_class_base: i32,
        flags: u32,
        exe_file: [u16; 260],
    }

    #[repr(C)]
    struct JobBasicLimit {
        per_process_user_time: i64,
        per_job_user_time: i64,
        limit_flags: u32,
        minimum_working_set: usize,
        maximum_working_set: usize,
        active_process_limit: u32,
        affinity: usize,
        priority_class: u32,
        scheduling_class: u32,
    }

    pub(super) struct OwnedHandle(*mut c_void);

    impl OwnedHandle {
        fn new(handle: *mut c_void) -> Option<Self> {
            if handle.is_null() || handle == INVALID_HANDLE {
                None
            } else {
                Some(Self(handle))
            }
        }
    }

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            // SAFETY: the handle was opened by this process and is still open.
            unsafe { CloseHandle(self.0) };
        }
    }

    // SAFETY: the handle is owned and moved with the session. It is not used
    // from two threads at the same time.
    unsafe impl Send for OwnedHandle {}

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcessId() -> u32;
        fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> *mut c_void;
        fn Process32FirstW(snapshot: *mut c_void, entry: *mut ProcessEntry32W) -> i32;
        fn Process32NextW(snapshot: *mut c_void, entry: *mut ProcessEntry32W) -> i32;
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
        fn QueryFullProcessImageNameW(
            process: *mut c_void,
            flags: u32,
            name: *mut u16,
            size: *mut u32,
        ) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
        fn CreateJobObjectW(attrs: *mut c_void, name: *const u16) -> *mut c_void;
        fn SetInformationJobObject(
            job: *mut c_void,
            class: i32,
            info: *const c_void,
            length: u32,
        ) -> i32;
        fn AssignProcessToJobObject(job: *mut c_void, process: *mut c_void) -> i32;
    }

    struct Proc {
        parent: u32,
        name: String,
    }

    pub(super) fn shell_ancestor() -> Option<String> {
        let table = process_table()?;
        let mut pid = unsafe { GetCurrentProcessId() };
        for _ in 0..16 {
            let parent = table.get(&pid)?.parent;
            if parent == 0 || parent == pid {
                return None;
            }
            let parent_info = table.get(&parent)?;
            if is_shell_name(&parent_info.name) {
                return process_image_path(parent).or_else(|| Some(parent_info.name.clone()));
            }
            pid = parent;
        }
        None
    }

    /// Puts `pid` in a job that ends every process in it when the handle is
    /// closed, so quitting StealthLingo also stops commands the shell started.
    pub(super) fn bind_kill_on_close(pid: u32) -> Option<OwnedHandle> {
        // SAFETY: the job information struct is zeroed plain data, and the
        // process handle is only passed to AssignProcessToJobObject.
        unsafe {
            let job = OwnedHandle::new(CreateJobObjectW(std::ptr::null_mut(), std::ptr::null()))?;
            let mut info: JobBasicLimit = std::mem::zeroed();
            info.limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                job.0,
                JOB_OBJECT_BASIC_LIMIT_INFORMATION,
                (&raw const info).cast(),
                std::mem::size_of::<JobBasicLimit>() as u32,
            ) == 0
            {
                return None;
            }
            let process =
                OwnedHandle::new(OpenProcess(PROCESS_TERMINATE | PROCESS_SET_QUOTA, 0, pid))?;
            if AssignProcessToJobObject(job.0, process.0) == 0 {
                return None;
            }
            Some(job)
        }
    }

    fn process_table() -> Option<HashMap<u32, Proc>> {
        // SAFETY: the snapshot handle is closed by OwnedHandle, and `entry`
        // is a writable ProcessEntry32W whose size field is set as required.
        unsafe {
            let snapshot = OwnedHandle::new(CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0))?;
            let mut entry: ProcessEntry32W = std::mem::zeroed();
            entry.size = std::mem::size_of::<ProcessEntry32W>() as u32;
            if Process32FirstW(snapshot.0, &mut entry) == 0 {
                return None;
            }
            let mut table = HashMap::new();
            loop {
                table.insert(
                    entry.pid,
                    Proc {
                        parent: entry.parent_pid,
                        name: wide_string(&entry.exe_file),
                    },
                );
                entry.size = std::mem::size_of::<ProcessEntry32W>() as u32;
                if Process32NextW(snapshot.0, &mut entry) == 0 {
                    break;
                }
            }
            Some(table)
        }
    }

    fn process_image_path(pid: u32) -> Option<String> {
        // SAFETY: the buffer is writable and its length is passed in characters.
        unsafe {
            let process = OwnedHandle::new(OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid))?;
            let mut buf = vec![0u16; 32_768];
            let mut len = buf.len() as u32;
            if QueryFullProcessImageNameW(process.0, 0, buf.as_mut_ptr(), &mut len) == 0 {
                return None;
            }
            Some(String::from_utf16_lossy(&buf[..len as usize]))
        }
    }

    fn wide_string(buf: &[u16]) -> String {
        let end = buf.iter().position(|&unit| unit == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..end])
    }

    #[cfg(test)]
    pub(super) fn current_process_is_listed() -> bool {
        let Some(table) = process_table() else {
            return false;
        };
        let pid = unsafe { GetCurrentProcessId() };
        table.contains_key(&pid)
    }
}

#[cfg(windows)]
fn is_shell_name(name: &str) -> bool {
    matches!(
        file_name(name).to_ascii_lowercase().as_str(),
        "powershell.exe"
            | "pwsh.exe"
            | "cmd.exe"
            | "bash.exe"
            | "zsh.exe"
            | "fish.exe"
            | "nu.exe"
            | "sh.exe"
            | "dash.exe"
            | "elvish.exe"
            | "xonsh.exe"
            | "ksh.exe"
            | "powershell"
            | "pwsh"
            | "cmd"
            | "bash"
            | "zsh"
            | "fish"
            | "nu"
            | "sh"
            | "dash"
            | "elvish"
            | "xonsh"
            | "ksh"
    )
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    fn press(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn esc_is_the_boss_key_and_is_not_forwarded() {
        let esc = press(KeyCode::Esc, KeyModifiers::empty());
        assert!(boss_key(&esc));
        assert!(shell_bytes(&esc).is_none());
        let repeat =
            KeyEvent::new_with_kind(KeyCode::Esc, KeyModifiers::empty(), KeyEventKind::Repeat);
        assert!(!boss_key(&repeat));
        assert!(shell_bytes(&repeat).is_none());
        let alt = press(KeyCode::Esc, KeyModifiers::ALT);
        assert!(!boss_key(&alt));
    }

    #[test]
    fn keys_become_terminal_sequences() {
        assert_eq!(
            shell_bytes(&press(KeyCode::Char('a'), KeyModifiers::empty())).unwrap(),
            b"a"
        );
        assert_eq!(
            shell_bytes(&press(KeyCode::Enter, KeyModifiers::empty())).unwrap(),
            b"\r"
        );
        assert_eq!(
            shell_bytes(&press(KeyCode::Backspace, KeyModifiers::empty())).unwrap(),
            b"\x7f"
        );
        assert_eq!(
            shell_bytes(&press(KeyCode::Up, KeyModifiers::empty())).unwrap(),
            b"\x1b[A"
        );
        assert_eq!(
            shell_bytes(&press(KeyCode::Up, KeyModifiers::SHIFT)).unwrap(),
            b"\x1b[1;2A"
        );
        assert_eq!(
            shell_bytes(&press(KeyCode::Left, KeyModifiers::CONTROL)).unwrap(),
            b"\x1b[1;5D"
        );
        assert_eq!(
            shell_bytes(&press(KeyCode::Char('c'), KeyModifiers::CONTROL)).unwrap(),
            b"\x03"
        );
        assert_eq!(
            shell_bytes(&press(KeyCode::Char('d'), KeyModifiers::CONTROL)).unwrap(),
            b"\x04"
        );
        assert_eq!(
            shell_bytes(&press(KeyCode::Char('b'), KeyModifiers::ALT)).unwrap(),
            b"\x1bb"
        );
    }

    #[test]
    fn shell_program_follows_the_parent_shell_on_windows_and_shell_on_unix() {
        let powershell = shell_launch(
            Host::Windows,
            None,
            Some(r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe"),
        );
        assert!(powershell.program.ends_with("powershell.exe"));
        assert_eq!(powershell.args, ["-NoLogo"]);

        let cmd = shell_launch(Host::Windows, Some("/usr/bin/bash"), Some("cmd.exe"));
        assert_eq!(cmd.program, "cmd.exe");
        assert!(cmd.args.is_empty());

        let fallback = shell_launch(Host::Windows, Some("/usr/bin/bash"), None);
        assert_eq!(fallback.program, "powershell.exe");
        assert_eq!(fallback.args, ["-NoLogo"]);

        let pwsh = shell_launch(
            Host::Windows,
            None,
            Some(r"C:\Program Files\PowerShell\7\pwsh.exe"),
        );
        assert!(pwsh.program.ends_with("pwsh.exe"));
        assert_eq!(pwsh.args, ["-NoLogo"]);

        let explicit = shell_launch(
            Host::Windows,
            Some(r"C:\Program Files\PowerShell\7\pwsh.exe"),
            None,
        );
        assert!(explicit.program.ends_with("pwsh.exe"));

        let zsh = shell_launch(Host::Unix, Some("/bin/zsh"), Some("powershell.exe"));
        assert_eq!(zsh.program, "/bin/zsh");
        assert!(zsh.args.is_empty());

        let sh = shell_launch(Host::Unix, Some("  "), None);
        assert_eq!(sh.program, "/bin/sh");
    }

    #[test]
    fn user_shell_names_a_program() {
        assert!(!user_shell().program.is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn process_snapshot_includes_this_process() {
        assert!(
            win32::current_process_is_listed(),
            "process snapshot failed; the process entry layout may be wrong"
        );
    }

    #[test]
    fn cursor_query_is_answered_and_not_shown() {
        let mut term = TermReply::new();
        let (display, reply) = term.push(b"ab\x1b[6n", 80, 24);
        assert_eq!(display, b"ab");
        assert_eq!(reply, b"\x1b[1;3R");

        let (split, none) = term.push(b"\x1b", 80, 24);
        assert!(split.is_empty() && none.is_empty());
        let (shown, reply) = term.push(b"[6n", 80, 24);
        assert!(shown.is_empty());
        assert_eq!(reply, b"\x1b[1;3R");

        let (colored, reply) = term.push(b"\x1b[31mx", 80, 24);
        assert_eq!(colored, b"\x1b[31mx");
        assert!(reply.is_empty());
    }

    fn output_contains(shell: &ShellSession, needle: &[u8]) -> bool {
        shell
            .gate
            .pending()
            .windows(needle.len())
            .any(|window| window == needle)
    }

    fn wait_for(shell: &ShellSession, needle: &[u8], timeout: Duration) -> bool {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if output_contains(shell, needle) {
                return true;
            }
            if !shell.alive.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(30));
                return output_contains(shell, needle);
            }
            thread::sleep(Duration::from_millis(20));
        }
        output_contains(shell, needle)
    }

    #[test]
    fn a_command_on_the_pty_is_captured() {
        let (program, args): (&str, &[&str]) = if cfg!(windows) {
            ("cmd.exe", &["/c", "echo hello-from-shell"])
        } else {
            ("/bin/sh", &["-c", "echo hello-from-shell"])
        };
        let shell = ShellSession::open(program, args, 80, 24).unwrap();
        assert!(
            wait_for(&shell, b"hello-from-shell", Duration::from_secs(5)),
            "output was {:?}",
            String::from_utf8_lossy(&shell.gate.pending())
        );
    }

    #[test]
    fn a_typed_command_reaches_the_shell() {
        let mut shell = if cfg!(windows) {
            ShellSession::open("cmd.exe", &[], 80, 24).unwrap()
        } else {
            ShellSession::open("/bin/sh", &[], 80, 24).unwrap()
        };
        shell.send(b"echo hello-typed\r").unwrap();
        assert!(
            wait_for(&shell, b"hello-typed", Duration::from_secs(5)),
            "output was {:?}",
            String::from_utf8_lossy(&shell.gate.pending())
        );
    }

    #[cfg(windows)]
    #[test]
    fn powershell_accepts_a_typed_command() {
        let mut shell =
            ShellSession::open("powershell.exe", &["-NoLogo", "-NoProfile"], 80, 24).unwrap();
        shell.send(b"Write-Output hello-from-pwsh\r").unwrap();
        assert!(
            wait_for(&shell, b"hello-from-pwsh", Duration::from_secs(20)),
            "output was {:?}",
            String::from_utf8_lossy(&shell.gate.pending())
        );
    }
}
