use std::fmt::Write as _;

use anyhow::Result;

use super::links::{self, LinkAction};
use super::{fetch_word, Context, Freshness};
use crate::dictionary::{Entry, Meaning};
use crate::time::format_local;

const BRIEF_MEANINGS: usize = 3;
const BRIEF_DEFINITIONS_PER_MEANING: usize = 3;
const MAX_ACCENTS: usize = 2;
const MAX_UNLABELLED_TRANSCRIPTIONS: usize = 3;

/// How much of an entry `lookup` prints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Detail {
    /// Pronunciation and the first few definitions of the main parts of speech.
    Brief,
    /// Every definition with examples, synonyms, antonyms and the source.
    Full,
}

pub fn run(ctx: &Context, word: &str, detail: Detail) -> Result<()> {
    let (cached, freshness) = fetch_word(ctx, word)?;
    if let Freshness::Fallback(err) = &freshness {
        println!("{err}");
        println!(
            "Showing the copy cached on {}.\n",
            format_local(cached.fetched_at)
        );
    }
    let links = links::enabled();
    print!("{}", render_entry(&cached.entry, detail, links));
    if let Some(note) = &cached.note {
        println!("\nYour note: {note}");
    }
    println!();
    println!("{}", next_steps(&cached.entry, cached.in_collection, links));
    Ok(())
}

/// Closing line: how to save (or remove) the word and how to hear it. With
/// `links`, saving and removing are Ctrl+clickable, and the listen hint is left
/// out when the pronunciation itself can be clicked.
pub fn next_steps(entry: &Entry, saved: bool, links: bool) -> String {
    if links {
        let word = entry.word.clone();
        let mut parts = vec![if saved {
            format!(
                "Saved in your word list · {}",
                links::hyperlink(&LinkAction::Remove(word).link(), "Remove")
            )
        } else {
            links::hyperlink(&LinkAction::Add(word).link(), "+ Add to word list")
        }];
        if !has_clickable_pronunciation(entry) {
            parts.extend(audio_hint(entry).map(|audio| format!("Listen: {audio}")));
        }
        return parts.join(" · ");
    }
    match (saved, audio_hint(entry)) {
        (true, Some(audio)) => format!("Saved in your word list. Listen: {audio}."),
        (true, None) => "Saved in your word list.".to_string(),
        (false, Some(audio)) => format!(
            "Save it: `stealthlingo add {}`. Listen: {audio}.",
            command_arg(&entry.word)
        ),
        (false, None) => format!(
            "Run `stealthlingo add {}` to save it for practice.",
            command_arg(&entry.word)
        ),
    }
}

/// Formats a dictionary entry for the terminal. With `links`, pronunciations
/// that have a recording are clickable terminal hyperlinks.
pub fn render_entry(entry: &Entry, detail: Detail, links: bool) -> String {
    render_entry_to_width(entry, detail, links, text_width())
}

/// [`render_entry`] with text wrapped to `width` columns.
pub fn render_entry_to_width(entry: &Entry, detail: Detail, links: bool, width: usize) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{}\n", entry.word);

    render_pronunciation(&mut out, entry, links);

    if entry.meanings.is_empty() {
        let _ = writeln!(out, "\n(no definitions available)");
    }
    let full = detail == Detail::Full;
    for meaning in shown_meanings(entry, detail) {
        let _ = writeln!(
            out,
            "\n{}",
            meaning
                .part_of_speech
                .as_deref()
                .unwrap_or("(unknown part of speech)")
        );
        let limit = if full {
            usize::MAX
        } else {
            BRIEF_DEFINITIONS_PER_MEANING
        };
        let definitions = &meaning.definitions[..meaning.definitions.len().min(limit)];
        let digits = definitions.len().to_string().len();
        let text_indent = " ".repeat(2 + digits + 2);
        for (i, d) in definitions.iter().enumerate() {
            out += &wrap(&d.definition, &format!("  {:>digits$}. ", i + 1), width);
            if let Some(example) = d.example.as_ref().filter(|_| full) {
                out += &wrap(example, &format!("{text_indent}e.g. "), width);
            }
        }
        if !full {
            continue;
        }
        for (label, words) in [
            ("Synonyms", &meaning.synonyms),
            ("Antonyms", &meaning.antonyms),
        ] {
            if !words.is_empty() {
                out += &wrap(&words.join(", "), &format!("  {label}: "), width);
            }
        }
    }

    if full {
        let source = match (entry.source_urls.first(), &entry.license) {
            (Some(url), Some(license)) => Some(format!("Source: {url} ({license})")),
            (Some(url), None) => Some(format!("Source: {url}")),
            (None, Some(license)) => Some(format!("License: {license}")),
            (None, None) => None,
        };
        if let Some(source) = source {
            let _ = writeln!(out, "\n{source}");
        }
    }
    out
}

/// Word-wraps `text` after `prefix`, indenting continuation lines to line up
/// with the first character of `text`. Words longer than a line are kept whole.
pub fn wrap(text: &str, prefix: &str, width: usize) -> String {
    let indent = prefix.chars().count();
    let mut out = String::from(prefix);
    let mut column = indent;
    let mut line_empty = true;
    for word in text.split_whitespace() {
        let len = word.chars().count();
        if !line_empty && column + 1 + len > width {
            out.push('\n');
            out.push_str(&" ".repeat(indent));
            column = indent;
            line_empty = true;
        }
        if !line_empty {
            out.push(' ');
            column += 1;
        }
        out.push_str(word);
        column += len;
        line_empty = false;
    }
    out.push('\n');
    out
}

/// Columns to wrap definitions at: the terminal width (one column spare so a
/// full line never triggers the terminal's own wrap), kept between 40 and 100
/// so lines stay readable. 80 when the width is unknown, e.g. when piped.
fn text_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|c| c.trim().parse::<usize>().ok())
        .or_else(console_width)
        .map_or(80, |w| w.saturating_sub(1).clamp(40, 100))
}

#[cfg(windows)]
fn console_width() -> Option<usize> {
    #![allow(dead_code)]
    use std::ffi::c_void;

    #[repr(C)]
    struct Coord {
        x: i16,
        y: i16,
    }
    #[repr(C)]
    struct SmallRect {
        left: i16,
        top: i16,
        right: i16,
        bottom: i16,
    }
    #[repr(C)]
    struct ScreenBufferInfo {
        size: Coord,
        cursor: Coord,
        attributes: u16,
        window: SmallRect,
        max_size: Coord,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetStdHandle(id: u32) -> *mut c_void;
        fn GetConsoleScreenBufferInfo(handle: *mut c_void, info: *mut ScreenBufferInfo) -> i32;
    }
    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;

    let mut info = ScreenBufferInfo {
        size: Coord { x: 0, y: 0 },
        cursor: Coord { x: 0, y: 0 },
        attributes: 0,
        window: SmallRect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
        max_size: Coord { x: 0, y: 0 },
    };
    // SAFETY: plain Win32 calls; `info` is a correctly laid out, writable struct
    // and the handle is only passed back to the console API.
    let ok = unsafe { GetConsoleScreenBufferInfo(GetStdHandle(STD_OUTPUT_HANDLE), &mut info) };
    let width = i32::from(info.window.right) - i32::from(info.window.left) + 1;
    (ok != 0 && width > 0).then_some(width as usize)
}

#[cfg(not(windows))]
fn console_width() -> Option<usize> {
    None
}

/// Meanings to print. The brief view skips `symbol` sections (such as ISO
/// language codes) unless nothing else is left, and keeps the first few.
fn shown_meanings(entry: &Entry, detail: Detail) -> Vec<&Meaning> {
    if detail == Detail::Full {
        return entry.meanings.iter().collect();
    }
    let is_symbol = |m: &&Meaning| m.part_of_speech.as_deref() == Some("symbol");
    let mut shown: Vec<&Meaning> = entry.meanings.iter().filter(|m| !is_symbol(m)).collect();
    if shown.is_empty() {
        shown = entry.meanings.iter().collect();
    }
    shown.truncate(BRIEF_MEANINGS);
    shown
}

/// Accents shown in order; when one of them is known, rarer accents and
/// unlabelled transcriptions are left out to keep the line short.
const MAIN_ACCENTS: [&str; 2] = ["UK", "US"];

/// One line such as `UK /njuː/ · US /nu/`, or `/a/, /b/` without accent labels.
/// With `links`, transcriptions that have a recording become clickable.
fn pronunciation_line(entry: &Entry, links: bool) -> Option<String> {
    let shown = shown_pronunciations(entry);
    let labelled = shown.first()?.0.is_some();
    let mut linked_unlabelled = false;
    let parts: Vec<String> = shown
        .iter()
        .map(|&(accent, text)| {
            let label = match accent {
                Some(accent) => format!("{accent} {text}"),
                None => text.to_string(),
            };
            let clickable = links
                && match accent {
                    Some(_) => has_recording(entry, accent),
                    None => entry.has_audio() && !std::mem::replace(&mut linked_unlabelled, true),
                };
            if clickable {
                links::hyperlink(&links::audio_link(&entry.word, accent), &label)
            } else {
                label
            }
        })
        .collect();
    Some(parts.join(if labelled { " · " } else { ", " }))
}

/// Whether [`pronunciation_line`] links at least one transcription to a recording.
fn has_clickable_pronunciation(entry: &Entry) -> bool {
    shown_pronunciations(entry)
        .iter()
        .any(|&(accent, _)| match accent {
            Some(_) => has_recording(entry, accent),
            None => entry.has_audio(),
        })
}

fn has_recording(entry: &Entry, accent: Option<&str>) -> bool {
    entry
        .phonetics
        .iter()
        .any(|p| p.audio_url.is_some() && p.accent.as_deref() == accent)
}

/// Transcriptions to show as `(accent, transcription)`: UK and US when known,
/// otherwise unlabelled ones (the general pronunciation), otherwise other
/// accents such as India.
fn shown_pronunciations(entry: &Entry) -> Vec<(Option<&str>, &str)> {
    let mut labelled: Vec<(&str, &str)> = Vec::new();
    let mut unlabelled: Vec<&str> = Vec::new();
    for p in &entry.phonetics {
        let Some(text) = p.text.as_deref() else {
            continue;
        };
        match p.accent.as_deref() {
            Some(accent) if !labelled.iter().any(|(a, _)| *a == accent) => {
                labelled.push((accent, text));
            }
            Some(_) => {}
            None if !unlabelled.contains(&text) => unlabelled.push(text),
            None => {}
        }
    }

    let rank = |accent: &str| {
        MAIN_ACCENTS
            .iter()
            .position(|m| *m == accent)
            .unwrap_or(MAIN_ACCENTS.len())
    };
    let is_main = |accent: &str| rank(accent) < MAIN_ACCENTS.len();
    if labelled.iter().any(|(a, _)| is_main(a)) {
        labelled.retain(|(a, _)| is_main(a));
        labelled.sort_by_key(|(a, _)| rank(a));
    } else if !unlabelled.is_empty() {
        unlabelled.truncate(MAX_UNLABELLED_TRANSCRIPTIONS);
        return unlabelled.into_iter().map(|t| (None, t)).collect();
    }
    labelled.truncate(MAX_ACCENTS);
    if !labelled.is_empty() {
        return labelled.into_iter().map(|(a, t)| (Some(a), t)).collect();
    }
    entry
        .phonetic
        .as_deref()
        .map(|t| (None, t))
        .into_iter()
        .collect()
}

fn render_pronunciation(out: &mut String, entry: &Entry, links: bool) {
    let line = pronunciation_line(entry, links);
    let _ = writeln!(
        out,
        "Pronunciation: {}",
        line.as_deref().unwrap_or("not available")
    );
}

/// `stealthlingo audio <word>` plus the accent of the recording it plays.
fn audio_hint(entry: &Entry) -> Option<String> {
    let audio = entry.audio()?;
    let accent = audio
        .accent
        .as_deref()
        .map(|a| format!(" ({a})"))
        .unwrap_or_default();
    Some(format!(
        "`stealthlingo audio {}`{accent}",
        command_arg(&entry.word)
    ))
}

/// Quotes a word for a copy-pasteable command line when it contains spaces.
fn command_arg(word: &str) -> String {
    if word.contains(char::is_whitespace) {
        format!("\"{word}\"")
    } else {
        word.to_string()
    }
}
