//! StealthLingo: a terminal-first vocabulary trainer backed by online dictionaries
//! (English Wiktionary) and a local SQLite cache.

pub mod audio;
pub mod cli;
pub mod commands;
pub mod config;
pub mod dictionary;
pub mod error;
pub mod learning;
pub mod storage;
pub mod time;
