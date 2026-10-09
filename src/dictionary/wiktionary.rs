//! English Wiktionary via the Wikimedia REST API (definitions and examples) plus
//! the page wikitext (IPA, pronunciation audio, synonyms and antonyms).
//! Audio files are served from Wikimedia Commons.

use std::collections::HashMap;

use reqwest::Url;
use serde::Deserialize;

use super::markup::html_to_text;
use super::models::{push_unique, Definition, Entry, Meaning, Phonetic};
use crate::error::DictionaryError;

pub const REST_BASE: &str = "https://en.wiktionary.org/api/rest_v1/page/definition";
pub const ACTION_API: &str = "https://en.wiktionary.org/w/api.php";
const COMMONS_FILE_PATH: &str = "https://commons.wikimedia.org/wiki/Special:FilePath";
pub const LICENSE: &str = "CC BY-SA 4.0";

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Usage {
    #[serde(rename = "partOfSpeech")]
    part_of_speech: Option<String>,
    definitions: Vec<RestDefinition>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RestDefinition {
    definition: String,
    #[serde(rename = "parsedExamples")]
    parsed_examples: Vec<ParsedExample>,
    examples: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ParsedExample {
    example: String,
}

/// Wiktionary page titles use underscores for spaces.
pub fn page_title(word: &str) -> String {
    word.split_whitespace().collect::<Vec<_>>().join("_")
}

pub fn page_url(word: &str) -> String {
    let mut url = Url::parse("https://en.wiktionary.org/wiki/").expect("static URL");
    url.path_segments_mut()
        .expect("static URL has a path")
        .pop_if_empty()
        .push(&page_title(word));
    url.to_string()
}

pub fn wikitext_url(word: &str) -> Url {
    Url::parse_with_params(
        ACTION_API,
        &[
            ("action", "parse"),
            ("page", &page_title(word)),
            ("prop", "wikitext"),
            ("format", "json"),
            ("formatversion", "2"),
            ("redirects", "1"),
        ],
    )
    .expect("static URL")
}

pub fn commons_audio_url(file: &str) -> String {
    let mut url = Url::parse(COMMONS_FILE_PATH).expect("static URL");
    url.path_segments_mut()
        .expect("static URL has a path")
        .push(&page_title(file));
    url.to_string()
}

/// Maps the REST definition response to an entry or a classified error.
pub fn interpret_definitions(
    word: &str,
    status: u16,
    body: &str,
) -> Result<Entry, DictionaryError> {
    match status {
        200..=299 => parse_definitions(word, body),
        404 => Err(DictionaryError::NotFound(word.trim().to_string())),
        other => Err(DictionaryError::Server(other)),
    }
}

/// Parses the English section of a REST definition response.
pub fn parse_definitions(word: &str, body: &str) -> Result<Entry, DictionaryError> {
    let mut by_language: HashMap<String, Vec<Usage>> =
        serde_json::from_str(body).map_err(|e| DictionaryError::Parse(e.to_string()))?;
    let usages = by_language.remove("en").unwrap_or_default();

    let mut entry = Entry {
        word: word.trim().to_string(),
        source_urls: vec![page_url(word)],
        license: Some(LICENSE.to_string()),
        ..Entry::default()
    };
    for usage in usages {
        let definitions: Vec<Definition> = usage
            .definitions
            .into_iter()
            .filter_map(|d| {
                let definition = html_to_text(&d.definition);
                if definition.is_empty() {
                    return None;
                }
                let example = d
                    .parsed_examples
                    .iter()
                    .map(|e| html_to_text(&e.example))
                    .chain(d.examples.iter().map(|e| html_to_text(e)))
                    .find(|e| !e.is_empty());
                Some(Definition {
                    definition,
                    example,
                })
            })
            .collect();
        if definitions.is_empty() {
            continue;
        }
        entry.meanings.push(Meaning {
            part_of_speech: usage
                .part_of_speech
                .map(|p| p.trim().to_lowercase())
                .filter(|p| !p.is_empty()),
            definitions,
            ..Meaning::default()
        });
    }
    if entry.meanings.is_empty() {
        return Err(DictionaryError::NotFound(word.trim().to_string()));
    }
    Ok(entry)
}

/// Pulls the wikitext out of an `action=parse` response.
pub fn extract_wikitext(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    value
        .get("parse")?
        .get("wikitext")?
        .as_str()
        .map(str::to_string)
}

/// Pronunciation and related-word data found in the English section.
#[derive(Debug, Default, PartialEq)]
pub struct PageExtras {
    pub ipa: Vec<String>,
    pub audio_files: Vec<String>,
    /// Synonyms and antonyms keyed by lowercase part of speech.
    pub synonyms: Vec<(String, Vec<String>)>,
    pub antonyms: Vec<(String, Vec<String>)>,
}

const PARTS_OF_SPEECH: &[&str] = &[
    "noun",
    "proper noun",
    "verb",
    "adjective",
    "adverb",
    "interjection",
    "pronoun",
    "preposition",
    "conjunction",
    "determiner",
    "numeral",
    "particle",
    "phrase",
    "prepositional phrase",
    "proverb",
    "prefix",
    "suffix",
    "article",
    "contraction",
];

fn english_section(wikitext: &str) -> Option<String> {
    let mut lines = wikitext.lines();
    lines.find(|line| line.trim() == "==English==")?;
    let section: Vec<&str> = lines
        .take_while(|line| {
            let t = line.trim_start();
            !t.starts_with("==") || t.starts_with("===")
        })
        .collect();
    Some(section.join("\n"))
}

/// Arguments of every `{{name|...}}` template in `line` whose name is in `names`.
fn templates<'a>(line: &'a str, names: &[&str]) -> Vec<Vec<&'a str>> {
    let mut found = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else { break };
        let inner = &after[..end];
        let mut parts = inner.split('|');
        if let Some(name) = parts.next() {
            if names.contains(&name.trim()) {
                found.push(parts.collect());
            }
        }
        rest = &after[end + 2..];
    }
    found
}

/// Positional (non `key=value`) arguments after the language code.
fn positional<'a>(args: &[&'a str]) -> Vec<&'a str> {
    args.iter()
        .skip(1)
        .map(|a| a.trim())
        .filter(|a| !a.is_empty() && !a.contains('='))
        .collect()
}

fn related_words(args: &[&str]) -> Vec<String> {
    positional(args)
        .into_iter()
        .filter(|w| !w.starts_with("Thesaurus:"))
        .map(|w| w.split('<').next().unwrap_or(w).trim().to_string())
        .filter(|w| !w.is_empty())
        .collect()
}

fn add_related(groups: &mut Vec<(String, Vec<String>)>, pos: &str, words: Vec<String>) {
    match groups.iter_mut().find(|(p, _)| p == pos) {
        Some((_, existing)) => push_unique(existing, words),
        None => {
            let mut list = Vec::new();
            push_unique(&mut list, words);
            groups.push((pos.to_string(), list));
        }
    }
}

pub fn parse_wikitext(wikitext: &str) -> PageExtras {
    let mut extras = PageExtras::default();
    let Some(section) = english_section(wikitext) else {
        return extras;
    };
    let mut current_pos = String::new();
    for line in section.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("===") {
            let heading = trimmed.trim_matches('=').trim().to_lowercase();
            if PARTS_OF_SPEECH.contains(&heading.as_str()) {
                current_pos = heading;
            }
            continue;
        }
        for args in templates(line, &["IPA"]) {
            if args.first().map(|l| l.trim()) == Some("en") {
                push_unique(
                    &mut extras.ipa,
                    positional(&args).into_iter().map(str::to_string),
                );
            }
        }
        for args in templates(line, &["audio"]) {
            if args.first().map(|l| l.trim()) == Some("en") {
                if let Some(file) = args.get(1).map(|f| f.trim()).filter(|f| !f.is_empty()) {
                    push_unique(&mut extras.audio_files, [file.to_string()]);
                }
            }
        }
        if current_pos.is_empty() {
            continue;
        }
        for args in templates(line, &["syn", "synonyms"]) {
            add_related(&mut extras.synonyms, &current_pos, related_words(&args));
        }
        for args in templates(line, &["ant", "antonyms"]) {
            add_related(&mut extras.antonyms, &current_pos, related_words(&args));
        }
    }
    extras
}

/// Merges pronunciation, audio and related words from the page into `entry`.
pub fn apply_extras(entry: &mut Entry, extras: PageExtras) {
    // Wiktionary does not tie recordings to a specific transcription, so they
    // are kept as separate phonetics.
    for ipa in &extras.ipa {
        entry.push_phonetic(Phonetic {
            text: Some(ipa.clone()),
            audio_url: None,
        });
    }
    for file in &extras.audio_files {
        entry.push_phonetic(Phonetic {
            text: None,
            audio_url: Some(commons_audio_url(file)),
        });
    }
    if entry.phonetic.is_none() {
        entry.phonetic = extras.ipa.first().cloned();
    }
    let mut seen: Vec<String> = Vec::new();
    for meaning in &mut entry.meanings {
        let Some(pos) = meaning.part_of_speech.clone() else {
            continue;
        };
        if seen.contains(&pos) {
            continue;
        }
        if let Some((_, words)) = extras.synonyms.iter().find(|(p, _)| *p == pos) {
            push_unique(&mut meaning.synonyms, words.iter().cloned());
        }
        if let Some((_, words)) = extras.antonyms.iter().find(|(p, _)| *p == pos) {
            push_unique(&mut meaning.antonyms, words.iter().cloned());
        }
        seen.push(pos);
    }
}
