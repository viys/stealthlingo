//! Clickable `lookup` output. `stealthlingo links install` registers the
//! `stealthlingo://` URL scheme with the operating system (Windows only), and
//! `lookup` then prints terminal hyperlinks (OSC 8) such as
//! `stealthlingo://audio/hostel?accent=US` or `stealthlingo://add/hostel`.
//! Ctrl+clicking one makes the OS run the windowless `stealthlingo-link`
//! program, which plays the recording or adds or removes the word.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context as _, Result};
use chrono::Utc;
use reqwest::Url;

use super::{cached_or_fetch, Context};
use crate::dictionary::normalize_headword;

pub const SCHEME: &str = "stealthlingo";
const HANDLER_NAME: &str = "stealthlingo-link";

/// What a clicked link asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkAction {
    /// Play the recording, in `accent` when given.
    Audio {
        word: String,
        accent: Option<String>,
    },
    /// Save the word to the study list.
    Add(String),
    /// Remove the word from the study list.
    Remove(String),
}

impl LinkAction {
    fn host(&self) -> &'static str {
        match self {
            Self::Audio { .. } => "audio",
            Self::Add(_) => "add",
            Self::Remove(_) => "remove",
        }
    }

    fn word(&self) -> &str {
        match self {
            Self::Audio { word, .. } | Self::Add(word) | Self::Remove(word) => word,
        }
    }

    /// The link, e.g. `stealthlingo://add/ice%20cream`.
    pub fn link(&self) -> String {
        let mut url = Url::parse(&format!("{SCHEME}://{}/", self.host())).expect("static URL");
        url.path_segments_mut()
            .expect("URL with a host has a path")
            .pop_if_empty()
            .push(self.word());
        if let Self::Audio {
            accent: Some(accent),
            ..
        } = self
        {
            url.query_pairs_mut().append_pair("accent", accent);
        }
        url.to_string()
    }

    /// Reads a link made by [`LinkAction::link`].
    pub fn parse(link: &str) -> Result<Self> {
        let url = Url::parse(link.trim()).with_context(|| format!("invalid link {link}"))?;
        if url.scheme() != SCHEME {
            bail!("not a StealthLingo link: {link}");
        }
        let word = url
            .path_segments()
            .and_then(|mut segments| segments.next())
            .filter(|s| !s.is_empty())
            .map(percent_decode)
            .ok_or_else(|| anyhow!("the link has no word: {link}"))?;
        match url.host_str() {
            Some("audio") => Ok(Self::Audio {
                word,
                accent: url
                    .query_pairs()
                    .find(|(key, _)| key == "accent")
                    .map(|(_, value)| value.into_owned()),
            }),
            Some("add") => Ok(Self::Add(word)),
            Some("remove") => Ok(Self::Remove(word)),
            _ => bail!("unknown StealthLingo link: {link}"),
        }
    }
}

/// `stealthlingo://audio/<word>`, with `?accent=<accent>` when given.
pub fn audio_link(word: &str, accent: Option<&str>) -> String {
    LinkAction::Audio {
        word: word.to_string(),
        accent: accent.map(str::to_string),
    }
    .link()
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                i += 3;
            }
            (byte, _) => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Wraps `text` in an OSC 8 terminal hyperlink to `url`.
pub fn hyperlink(url: &str, text: &str) -> String {
    format!("\x1b]8;;{url}\x1b\\{text}\x1b]8;;\x1b\\")
}

/// True when `lookup` should print clickable links: the scheme is registered
/// to a program that still exists, and stdout is a terminal that renders
/// hyperlinks.
pub fn enabled() -> bool {
    terminal_shows_links()
        && registered_handler()
            .ok()
            .flatten()
            .and_then(|command| registered_program(&command))
            .is_some_and(|program| program.is_file())
}

/// Program path in a registered command such as `"C:\...\stealthlingo-link.exe" "%1"`.
fn registered_program(command: &str) -> Option<PathBuf> {
    command
        .split('"')
        .nth(1)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
}

/// Whether stdout is a terminal known to render OSC 8 hyperlinks. Terminals
/// that do not would print the escape codes as garbage, so unknown ones are
/// treated as unsupported. `FORCE_HYPERLINK=1` or `=0` overrides detection.
pub fn terminal_shows_links() -> bool {
    use std::io::IsTerminal;

    let env = |name: &str| std::env::var(name).ok();
    if let Some(force) = env("FORCE_HYPERLINK") {
        return force.trim() != "0";
    }
    if !std::io::stdout().is_terminal() {
        return false;
    }
    // Windows Terminal, Konsole, DomTerm.
    if env("WT_SESSION").is_some() || env("KONSOLE_VERSION").is_some() || env("DOMTERM").is_some() {
        return true;
    }
    // VS Code, Cursor and other xterm.js-based or modern terminals.
    if env("TERM_PROGRAM").is_some_and(|program| {
        matches!(
            program.as_str(),
            "vscode" | "iTerm.app" | "WezTerm" | "ghostty" | "Hyper" | "zed"
        )
    }) {
        return true;
    }
    // GNOME Terminal and other VTE terminals from 0.50 on.
    if env("VTE_VERSION").and_then(|v| v.parse::<u32>().ok()) >= Some(5000) {
        return true;
    }
    env("TERM").is_some_and(|term| matches!(term.as_str(), "xterm-kitty" | "alacritty"))
}

/// Carries out a clicked link.
pub fn open(ctx: &Context, link: &str) -> Result<()> {
    match LinkAction::parse(link)? {
        LinkAction::Audio { word, accent } => {
            let (cached, _) = cached_or_fetch(ctx, &word)?;
            let accent = accent.as_deref().unwrap_or(&ctx.config.accent);
            let url = cached
                .entry
                .audio_in(Some(accent))
                .and_then(|p| p.audio_url.as_deref())
                .ok_or_else(|| anyhow!("no pronunciation audio for \"{word}\""))?;
            ctx.audio_player()?
                .play(&normalize_headword(&cached.entry.word), url)
        }
        LinkAction::Add(word) => {
            let (cached, _) = cached_or_fetch(ctx, &word)?;
            ctx.db.add_to_collection(cached.id, None, Utc::now())?;
            Ok(())
        }
        LinkAction::Remove(word) => {
            ctx.db
                .archive_from_collection(&normalize_headword(&word), Utc::now())?;
            Ok(())
        }
    }
}

/// The windowless handler program installed next to `stealthlingo`.
fn handler_path() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("could not locate the stealthlingo program")?;
    let handler = exe.with_file_name(format!("{HANDLER_NAME}{}", std::env::consts::EXE_SUFFIX));
    if !handler.is_file() {
        bail!(
            "{} was not found next to {}; reinstall with `cargo install --path .`",
            handler.display(),
            exe.display()
        );
    }
    Ok(handler)
}

pub fn install() -> Result<()> {
    let handler = handler_path()?;
    register(&handler)?;
    println!("Registered {SCHEME}:// links to {}.", handler.display());
    println!(
        "In `lookup`, Ctrl+click a pronunciation to play it, or the last line to add or remove the word."
    );
    Ok(())
}

pub fn uninstall() -> Result<()> {
    if unregister()? {
        println!("Removed the {SCHEME}:// link handler.");
    } else {
        println!("The {SCHEME}:// link handler was not installed.");
    }
    Ok(())
}

pub fn status() -> Result<()> {
    match registered_handler()? {
        Some(command) => {
            println!("{SCHEME}:// links are handled by: {command}");
            let program = registered_program(&command);
            if !program.as_deref().is_some_and(Path::is_file) {
                println!(
                    "That program no longer exists, so `lookup` prints no links; \
                     run `stealthlingo links install` again."
                );
            } else if let Ok(current) = handler_path() {
                if program.as_deref() != Some(current.as_path()) {
                    println!(
                        "This stealthlingo uses {}; run `stealthlingo links install` to switch to it.",
                        current.display()
                    );
                }
            }
        }
        None => println!("{SCHEME}:// links are not set up; run `stealthlingo links install`."),
    }
    let terminal = if terminal_shows_links() {
        "yes"
    } else {
        "no (or not detected; set FORCE_HYPERLINK=1 to override)"
    };
    println!("This terminal shows clickable links: {terminal}");
    Ok(())
}

/// Per-user URL scheme registration; needs no administrator rights.
#[cfg(windows)]
const CLASS_KEY: &str = r"HKCU\Software\Classes\stealthlingo";

/// Runs Windows' `reg.exe`; returns whether it succeeded and its stdout.
#[cfg(windows)]
fn reg(args: &[&str]) -> Result<(bool, String)> {
    let output = std::process::Command::new("reg")
        .args(args)
        .output()
        .context("could not run reg.exe")?;
    Ok((
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    ))
}

#[cfg(windows)]
fn register(handler: &std::path::Path) -> Result<()> {
    let command_key = format!(r"{CLASS_KEY}\shell\open\command");
    let command = format!("\"{}\" \"%1\"", handler.display());
    let steps: [&[&str]; 3] = [
        &["add", CLASS_KEY, "/ve", "/d", "URL:StealthLingo", "/f"],
        &["add", CLASS_KEY, "/v", "URL Protocol", "/d", "", "/f"],
        &["add", &command_key, "/ve", "/d", &command, "/f"],
    ];
    for args in steps {
        if !reg(args)?.0 {
            bail!("could not write {CLASS_KEY} in the registry");
        }
    }
    Ok(())
}

#[cfg(windows)]
fn unregister() -> Result<bool> {
    if registered_handler()?.is_none() {
        return Ok(false);
    }
    if !reg(&["delete", CLASS_KEY, "/f"])?.0 {
        bail!("could not remove {CLASS_KEY} from the registry");
    }
    Ok(true)
}

/// Command registered for the scheme, if any.
#[cfg(windows)]
fn registered_handler() -> Result<Option<String>> {
    let (found, stdout) = reg(&["query", &format!(r"{CLASS_KEY}\shell\open\command"), "/ve"])?;
    if !found {
        return Ok(None);
    }
    // Output line: `    (Default)    REG_SZ    "C:\...\stealthlingo-link.exe" "%1"`
    Ok(stdout
        .lines()
        .find_map(|line| line.split_once("REG_SZ"))
        .map(|(_, value)| value.trim().to_string())
        .filter(|value| !value.is_empty()))
}

#[cfg(not(windows))]
fn register(_handler: &std::path::Path) -> Result<()> {
    bail!("clickable pronunciations are only supported on Windows for now")
}

#[cfg(not(windows))]
fn unregister() -> Result<bool> {
    Ok(false)
}

#[cfg(not(windows))]
fn registered_handler() -> Result<Option<String>> {
    Ok(None)
}
