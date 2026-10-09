//! The internal, source-independent dictionary entry model.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Entry {
    pub word: String,
    pub phonetic: Option<String>,
    pub phonetics: Vec<Phonetic>,
    pub meanings: Vec<Meaning>,
    pub source_urls: Vec<String>,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Phonetic {
    pub text: Option<String>,
    pub audio_url: Option<String>,
    /// Short accent name such as "UK" or "US" (see [`accent_name`]).
    pub accent: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Meaning {
    pub part_of_speech: Option<String>,
    pub definitions: Vec<Definition>,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Definition {
    pub definition: String,
    pub example: Option<String>,
}

/// Canonical key used to identify a headword locally.
pub fn normalize_headword(word: &str) -> String {
    word.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Turns protocol-relative audio links (`//host/x.mp3`) into HTTPS and drops blanks.
pub fn normalize_audio_url(raw: &str) -> Option<String> {
    let url = raw.trim();
    if url.is_empty() {
        None
    } else if let Some(rest) = url.strip_prefix("//") {
        Some(format!("https://{rest}"))
    } else {
        Some(url.to_string())
    }
}

/// Maps a dialect label used by dictionaries ("RP", "GA", "en-GB", ...) to a
/// short accent name. Labels that are not accents (e.g. "cot-caught") give `None`.
pub fn accent_name(label: &str) -> Option<&'static str> {
    let key = label.trim().to_lowercase().replace('_', "-");
    let name = match key.as_str() {
        "uk"
        | "gb"
        | "en-gb"
        | "rp"
        | "ssb"
        | "british"
        | "britain"
        | "england"
        | "received pronunciation"
        | "standard southern british" => "UK",
        "us" | "en-us" | "ga" | "genam" | "usa" | "american" | "general american" => "US",
        "ca" | "canada" | "canadian" | "cane" => "Canada",
        "au" | "aus" | "aue" | "australia" | "australian" | "general australian" => "Australia",
        "nz" | "new zealand" => "New Zealand",
        "ie" | "ireland" | "irish" => "Ireland",
        "indic" | "india" | "indian" | "inde" | "indian english" | "general indian english" => {
            "India"
        }
        "scotland" | "scottish" | "sce" => "Scotland",
        "south africa" | "sae" => "South Africa",
        _ => return None,
    };
    Some(name)
}

/// Accent of a recording guessed from its file name, as in `hello-uk.mp3`,
/// `hello--_gb_1.mp3` or `En-us-hostel.ogg`.
pub fn accent_from_audio_url(url: &str) -> Option<&'static str> {
    let file = url.rsplit('/').next()?;
    let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
    // The first token is the word itself or a language code.
    stem.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .skip(1)
        .map(str::to_lowercase)
        .find(|t| matches!(t.as_str(), "uk" | "gb" | "us" | "au" | "ca" | "nz" | "ie"))
        .and_then(|t| accent_name(&t))
}

pub(crate) fn non_blank(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

pub(crate) fn push_unique(target: &mut Vec<String>, items: impl IntoIterator<Item = String>) {
    for item in items {
        let item = item.trim().to_string();
        if !item.is_empty() && !target.contains(&item) {
            target.push(item);
        }
    }
}

impl Entry {
    /// The pronunciation whose recording is played: a UK or US recording when
    /// available, otherwise the first one.
    pub fn audio(&self) -> Option<&Phonetic> {
        let mut recordings = self.phonetics.iter().filter(|p| p.audio_url.is_some());
        ["UK", "US"]
            .iter()
            .find_map(|accent| {
                recordings
                    .clone()
                    .find(|p| p.accent.as_deref() == Some(accent))
            })
            .or_else(|| recordings.next())
    }

    /// First usable pronunciation audio URL, if any.
    pub fn audio_url(&self) -> Option<&str> {
        self.audio().and_then(|p| p.audio_url.as_deref())
    }

    pub fn has_audio(&self) -> bool {
        self.audio_url().is_some()
    }

    /// First definition together with its part of speech.
    pub fn primary_definition(&self) -> Option<(Option<&str>, &Definition)> {
        self.meanings.iter().find_map(|m| {
            m.definitions
                .first()
                .map(|d| (m.part_of_speech.as_deref(), d))
        })
    }

    pub fn first_example(&self) -> Option<&str> {
        self.meanings
            .iter()
            .flat_map(|m| m.definitions.iter())
            .find_map(|d| d.example.as_deref())
    }

    pub fn parts_of_speech(&self) -> Vec<&str> {
        let mut parts = Vec::new();
        for pos in self
            .meanings
            .iter()
            .filter_map(|m| m.part_of_speech.as_deref())
        {
            if !parts.contains(&pos) {
                parts.push(pos);
            }
        }
        parts
    }

    /// One-line summary such as "adjective: lasting for a very short time".
    pub fn short_summary(&self) -> String {
        match self.primary_definition() {
            Some((Some(pos), d)) => format!("{pos}: {}", d.definition),
            Some((None, d)) => d.definition.clone(),
            None => "(no definition available)".to_string(),
        }
    }

    /// Adds a phonetic unless an identical one is already present.
    pub(crate) fn push_phonetic(&mut self, phonetic: Phonetic) {
        if (phonetic.text.is_some() || phonetic.audio_url.is_some())
            && !self.phonetics.contains(&phonetic)
        {
            self.phonetics.push(phonetic);
        }
    }
}
