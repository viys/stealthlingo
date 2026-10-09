//! Decodes data cached by v0.1, which stored raw Free Dictionary API
//! (<https://dictionaryapi.dev/>) responses. Only used for old database rows
//! and version 1 backups; nothing is fetched from that service any more.

use serde::Deserialize;

use super::models::{
    accent_from_audio_url, non_blank, normalize_audio_url, push_unique, Definition, Entry, Meaning,
    Phonetic,
};
use crate::error::DictionaryError;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct ApiEntry {
    word: Option<String>,
    phonetic: Option<String>,
    phonetics: Vec<ApiPhonetic>,
    meanings: Vec<ApiMeaning>,
    license: Option<ApiLicense>,
    #[serde(rename = "sourceUrls")]
    source_urls: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct ApiPhonetic {
    text: Option<String>,
    audio: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct ApiMeaning {
    #[serde(rename = "partOfSpeech")]
    part_of_speech: Option<String>,
    definitions: Vec<ApiDefinition>,
    synonyms: Vec<String>,
    antonyms: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct ApiDefinition {
    definition: Option<String>,
    example: Option<String>,
    synonyms: Vec<String>,
    antonyms: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct ApiLicense {
    name: Option<String>,
}

/// Parses a stored response body. The API returned one array element per
/// etymology/homograph; they are merged into a single entry.
pub fn parse(requested_word: &str, body: &str) -> Result<Entry, DictionaryError> {
    let api_entries: Vec<ApiEntry> =
        serde_json::from_str(body).map_err(|e| DictionaryError::Parse(e.to_string()))?;
    if api_entries.is_empty() {
        return Err(DictionaryError::NotFound(requested_word.trim().to_string()));
    }

    let word = api_entries
        .iter()
        .find_map(|e| non_blank(e.word.clone()))
        .unwrap_or_else(|| requested_word.trim().to_string());
    let mut entry = Entry {
        word,
        ..Entry::default()
    };

    for api in api_entries {
        if entry.phonetic.is_none() {
            entry.phonetic = non_blank(api.phonetic);
        }
        for p in api.phonetics {
            let audio_url = p.audio.as_deref().and_then(normalize_audio_url);
            let accent = audio_url
                .as_deref()
                .and_then(accent_from_audio_url)
                .map(str::to_string);
            entry.push_phonetic(Phonetic {
                text: non_blank(p.text),
                audio_url,
                accent,
            });
        }
        for m in api.meanings {
            let mut synonyms = Vec::new();
            let mut antonyms = Vec::new();
            push_unique(&mut synonyms, m.synonyms);
            push_unique(&mut antonyms, m.antonyms);
            let mut definitions = Vec::new();
            for d in m.definitions {
                push_unique(&mut synonyms, d.synonyms);
                push_unique(&mut antonyms, d.antonyms);
                if let Some(definition) = non_blank(d.definition) {
                    definitions.push(Definition {
                        definition,
                        example: non_blank(d.example),
                    });
                }
            }
            if definitions.is_empty() && synonyms.is_empty() && antonyms.is_empty() {
                continue;
            }
            entry.meanings.push(Meaning {
                part_of_speech: non_blank(m.part_of_speech),
                definitions,
                synonyms,
                antonyms,
            });
        }
        push_unique(&mut entry.source_urls, api.source_urls);
        if entry.license.is_none() {
            entry.license = api.license.and_then(|l| non_blank(l.name));
        }
    }

    if entry.phonetic.is_none() {
        entry.phonetic = entry.phonetics.iter().find_map(|p| p.text.clone());
    }
    Ok(entry)
}
