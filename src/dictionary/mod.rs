//! Dictionary sources. CLI commands only see [`Entry`], never remote JSON.

pub mod client;
pub mod free_dictionary;
pub mod markup;
pub mod merriam_webster;
pub mod models;
pub mod wiktionary;

use serde::{Deserialize, Serialize};

pub use client::{DictionaryClient, Fetched, SourceSettings};
pub use models::{normalize_audio_url, normalize_headword, Definition, Entry, Meaning, Phonetic};

/// Where dictionary data comes from. Selected with `config set dictionary_source`
/// or per command with `--source`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    /// English Wiktionary via the Wikimedia API, no key needed
    #[default]
    Wiktionary,
    /// Free Dictionary API (dictionaryapi.dev), no key needed
    FreeDictionary,
    /// Merriam-Webster Collegiate Dictionary, needs a free API key
    MerriamWebster,
}

impl Source {
    pub const ALL: [Source; 3] = [Self::Wiktionary, Self::FreeDictionary, Self::MerriamWebster];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::FreeDictionary => "free-dictionary",
            Self::Wiktionary => "wiktionary",
            Self::MerriamWebster => "merriam-webster",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::FreeDictionary => "Free Dictionary API",
            Self::Wiktionary => "Wiktionary",
            Self::MerriamWebster => "Merriam-Webster",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim().to_ascii_lowercase().replace('_', "-");
        Self::ALL.into_iter().find(|s| s.as_str() == value)
    }

    pub fn names() -> String {
        Self::ALL.map(Source::as_str).join(", ")
    }
}

/// Re-parses cached data: new rows store the normalized entry as JSON; rows
/// written before multi-source support only have the Free Dictionary response.
pub fn decode_cached(
    display_word: &str,
    raw_json: &str,
    entry_json: Option<&str>,
) -> Result<Entry, String> {
    match entry_json {
        Some(json) => serde_json::from_str(json).map_err(|e| e.to_string()),
        None => free_dictionary::parse(display_word, raw_json).map_err(|e| e.to_string()),
    }
}
