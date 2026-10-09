use std::fmt::Write as _;

use anyhow::Result;

use super::{fetch_word, Context, Freshness};
use crate::dictionary::Entry;
use crate::time::format_local;

const MAX_DEFINITIONS_PER_MEANING: usize = 6;
const MAX_PRONUNCIATIONS: usize = 3;

pub fn run(ctx: &Context, word: &str) -> Result<()> {
    let (cached, freshness) = fetch_word(ctx, word)?;
    if let Freshness::Fallback(err) = &freshness {
        println!("{err}");
        println!(
            "Showing the copy from {} cached on {}.\n",
            cached.source.label(),
            format_local(cached.fetched_at)
        );
    }
    print!("{}", render_entry(&cached.entry));
    println!("Dictionary: {}", cached.source.label());
    if let Some(note) = &cached.note {
        println!("\nYour note: {note}");
    }
    println!();
    if cached.in_collection {
        println!("Saved in your word list.");
    } else {
        println!(
            "Run `stealthlingo add {}` to save it for practice.",
            cached.entry.word
        );
    }
    Ok(())
}

/// Formats a dictionary entry for the terminal.
pub fn render_entry(entry: &Entry) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{}", entry.word);

    let mut pronunciations: Vec<String> = Vec::new();
    for p in &entry.phonetics {
        if let Some(text) = &p.text {
            if !pronunciations.contains(text) {
                pronunciations.push(text.clone());
            }
        }
    }
    if pronunciations.is_empty() {
        if let Some(phonetic) = &entry.phonetic {
            pronunciations.push(phonetic.clone());
        }
    }
    pronunciations.truncate(MAX_PRONUNCIATIONS);
    let audio = if entry.has_audio() {
        "audio: `stealthlingo audio`"
    } else {
        "no audio"
    };
    if pronunciations.is_empty() {
        let _ = writeln!(out, "Pronunciation: not available ({audio})");
    } else {
        let _ = writeln!(
            out,
            "Pronunciation: {} ({audio})",
            pronunciations.join(", ")
        );
    }

    if entry.meanings.is_empty() {
        let _ = writeln!(out, "\n(no definitions available)");
    }
    for meaning in &entry.meanings {
        let _ = writeln!(
            out,
            "\n{}",
            meaning
                .part_of_speech
                .as_deref()
                .unwrap_or("(unknown part of speech)")
        );
        for (i, d) in meaning
            .definitions
            .iter()
            .take(MAX_DEFINITIONS_PER_MEANING)
            .enumerate()
        {
            let _ = writeln!(out, "  {}. {}", i + 1, d.definition);
            if let Some(example) = &d.example {
                let _ = writeln!(out, "     Example: {example}");
            }
        }
        let hidden = meaning
            .definitions
            .len()
            .saturating_sub(MAX_DEFINITIONS_PER_MEANING);
        if hidden > 0 {
            let _ = writeln!(out, "  (+{hidden} more)");
        }
        if !meaning.synonyms.is_empty() {
            let _ = writeln!(out, "  Synonyms: {}", meaning.synonyms.join(", "));
        }
        if !meaning.antonyms.is_empty() {
            let _ = writeln!(out, "  Antonyms: {}", meaning.antonyms.join(", "));
        }
    }

    if !entry.source_urls.is_empty() || entry.license.is_some() {
        let _ = writeln!(out);
        if let Some(source) = entry.source_urls.first() {
            let _ = writeln!(out, "Source: {source}");
        }
        if let Some(license) = &entry.license {
            let _ = writeln!(out, "License: {license}");
        }
    }
    out
}
