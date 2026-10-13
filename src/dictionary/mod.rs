//! English Wiktionary lookups. CLI commands only see [`Entry`], never remote JSON.

pub mod client;
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
