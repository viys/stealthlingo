//! Merriam-Webster's Collegiate Dictionary API (<https://dictionaryapi.com/>).
//! Requires a free API key. Unknown words come back as a list of suggestions.

use reqwest::Url;
use serde_json::Value;

use super::markup::strip_mw_markup;
use super::models::{Definition, Entry, Meaning, Phonetic};
use crate::error::DictionaryError;

pub const API_BASE: &str = "https://www.dictionaryapi.com/api/v3/references/collegiate/json";
pub const REGISTER_URL: &str = "https://dictionaryapi.com/register/index";

pub fn missing_key_error() -> DictionaryError {
    DictionaryError::Config(format!(
        "Merriam-Webster needs a free API key: register at {REGISTER_URL} (Collegiate Dictionary), \
         then run `stealthlingo config set merriam_webster_key <KEY>`"
    ))
}

fn invalid_key_error() -> DictionaryError {
    DictionaryError::Config(
        "Merriam-Webster rejected the API key; check it with `stealthlingo config` and make sure \
         it is a Collegiate Dictionary key"
            .to_string(),
    )
}

pub fn entry_url(word: &str, key: &str) -> Url {
    let mut url = Url::parse(API_BASE).expect("static URL");
    url.path_segments_mut()
        .expect("static URL has a path")
        .push(word.trim());
    url.query_pairs_mut().append_pair("key", key);
    url
}

/// Audio file URL following Merriam-Webster's documented directory rules.
pub fn audio_url(file: &str) -> Option<String> {
    let file = file.trim();
    let first = file.chars().next()?;
    let subdir = if file.starts_with("bix") {
        "bix".to_string()
    } else if file.starts_with("gg") {
        "gg".to_string()
    } else if !first.is_ascii_alphabetic() {
        "number".to_string()
    } else {
        first.to_ascii_lowercase().to_string()
    };
    Some(format!(
        "https://media.merriam-webster.com/audio/prons/en/us/mp3/{subdir}/{file}.mp3"
    ))
}

pub fn interpret(word: &str, status: u16, body: &str) -> Result<Entry, DictionaryError> {
    match status {
        200..=299 => parse(word, body),
        401 | 403 => Err(invalid_key_error()),
        other => Err(DictionaryError::Server(other)),
    }
}

/// Headword without syllable dots (`ephem*er*al`) or homograph suffixes (`run:1`).
fn clean_headword(value: &str) -> String {
    let base = value.split(':').next().unwrap_or(value);
    base.replace('*', "").trim().to_string()
}

fn entry_matches(entry: &Value, word: &str) -> bool {
    let id = entry["meta"]["id"].as_str().map(clean_headword);
    let hw = entry["hwi"]["hw"].as_str().map(clean_headword);
    [id, hw]
        .into_iter()
        .flatten()
        .any(|h| h.eq_ignore_ascii_case(word))
}

fn stem_matches(entry: &Value, word: &str) -> bool {
    entry["meta"]["stems"].as_array().is_some_and(|stems| {
        stems
            .iter()
            .filter_map(Value::as_str)
            .any(|s| s.eq_ignore_ascii_case(word))
    })
}

/// First example sentence (`["vis", [{"t": ...}]]`) anywhere inside `value`.
fn find_example(value: &Value) -> Option<String> {
    match value {
        Value::Array(items) => {
            if items.first().and_then(Value::as_str) == Some("vis") {
                if let Some(text) = items
                    .get(1)
                    .and_then(Value::as_array)
                    .and_then(|v| v.first())
                    .and_then(|v| v["t"].as_str())
                {
                    let text = strip_mw_markup(text);
                    if !text.is_empty() {
                        return Some(text);
                    }
                }
            }
            items.iter().find_map(find_example)
        }
        Value::Object(map) => map.values().find_map(find_example),
        _ => None,
    }
}

pub fn parse(word: &str, body: &str) -> Result<Entry, DictionaryError> {
    let value: Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(_) if body.to_ascii_lowercase().contains("key") => return Err(invalid_key_error()),
        Err(e) => return Err(DictionaryError::Parse(e.to_string())),
    };
    let word = word.trim();
    let items = value
        .as_array()
        .ok_or_else(|| DictionaryError::Parse("expected a JSON array".to_string()))?;
    // Unknown words return an array of suggestion strings instead of entries.
    let entries: Vec<&Value> = items.iter().filter(|v| v.is_object()).collect();

    let mut selected: Vec<&Value> = entries
        .iter()
        .copied()
        .filter(|e| entry_matches(e, word))
        .collect();
    if selected.is_empty() {
        selected = entries
            .iter()
            .copied()
            .filter(|e| stem_matches(e, word))
            .collect();
    }
    if selected.is_empty() {
        return Err(DictionaryError::NotFound(word.to_string()));
    }

    let display = selected[0]["meta"]["id"]
        .as_str()
        .or_else(|| selected[0]["hwi"]["hw"].as_str())
        .map(clean_headword)
        .filter(|w| !w.is_empty())
        .unwrap_or_else(|| word.to_string());
    let mut entry = Entry {
        source_urls: vec![format!(
            "https://www.merriam-webster.com/dictionary/{}",
            display.replace(' ', "%20")
        )],
        word: display,
        license: Some("Merriam-Webster's Collegiate Dictionary".to_string()),
        ..Entry::default()
    };

    for item in selected {
        if let Some(prs) = item["hwi"]["prs"].as_array() {
            for pr in prs {
                let text = pr["mw"]
                    .as_str()
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                    .map(|t| format!("\\{t}\\"));
                let audio = pr["sound"]["audio"].as_str().and_then(audio_url);
                entry.push_phonetic(Phonetic {
                    text,
                    audio_url: audio,
                });
            }
        }
        let mut definitions: Vec<Definition> = item["shortdef"]
            .as_array()
            .map(|defs| {
                defs.iter()
                    .filter_map(Value::as_str)
                    .map(strip_mw_markup)
                    .filter(|d| !d.is_empty())
                    .map(|definition| Definition {
                        definition,
                        example: None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        if definitions.is_empty() {
            continue;
        }
        definitions[0].example = find_example(&item["def"]);
        entry.meanings.push(Meaning {
            part_of_speech: item["fl"]
                .as_str()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            definitions,
            ..Meaning::default()
        });
    }
    entry.phonetic = entry.phonetics.iter().find_map(|p| p.text.clone());
    if entry.meanings.is_empty() {
        return Err(DictionaryError::NotFound(word.to_string()));
    }
    Ok(entry)
}
