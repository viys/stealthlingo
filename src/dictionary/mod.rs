//! English Wiktionary lookups. CLI commands only see [`Entry`], never remote JSON.

pub mod client;
pub mod legacy;
pub mod markup;
pub mod models;
pub mod wiktionary;

pub use client::{DictionaryClient, Endpoints, Fetched};
pub use models::{
    accent_from_audio_url, accent_name, clean_word, normalize_audio_url, normalize_headword,
    Definition, Entry, Meaning, Phonetic,
};

/// Value of the `source` column for entries fetched from Wiktionary. Older
/// rows may name a dictionary StealthLingo no longer uses.
pub const SOURCE: &str = "wiktionary";

/// `source` of Wiktionary entries whose pronunciation data could not be
/// fetched; they are refreshed the next time they are used.
pub const PARTIAL_SOURCE: &str = "wiktionary-partial";

/// Re-parses cached data: rows store the normalized entry as JSON; rows
/// written by v0.1 only have the Free Dictionary response.
pub fn decode_cached(
    display_word: &str,
    raw_json: &str,
    entry_json: Option<&str>,
) -> Result<Entry, String> {
    match entry_json {
        Some(json) => serde_json::from_str(json).map_err(|e| e.to_string()),
        None => legacy::parse(display_word, raw_json).map_err(|e| e.to_string()),
    }
}
