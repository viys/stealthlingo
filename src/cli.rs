use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::dictionary::Source;

/// Quick, low-distraction vocabulary practice in your terminal.
///
/// Run without a command to start today's practice: due reviews first,
/// otherwise a short session with new words.
#[derive(Debug, Parser)]
#[command(name = "stealthlingo", version, about, long_about)]
pub struct Cli {
    /// Dictionary to query for this command (overrides `config set dictionary_source`)
    #[arg(long, global = true, value_enum)]
    pub source: Option<Source>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Look up a word in the Free Dictionary API
    Lookup { word: String },
    /// Save a word to your study list
    Add {
        word: String,
        /// Personal note or translation shown as an extra hint
        #[arg(long, short)]
        note: Option<String>,
    },
    /// Remove a word from your study list
    Remove { word: String },
    /// List saved words
    Words,
    /// Search saved words and personal notes
    Search { query: String },
    /// Start a study session
    Study(StudyArgs),
    /// Review words that are due now
    Review {
        /// Stop after this many answers
        #[arg(long, short)]
        count: Option<usize>,
        /// Stop after this many minutes
        #[arg(long, short)]
        minutes: Option<u32>,
    },
    /// Show learning statistics
    Stats,
    /// Play the pronunciation of a word
    Audio { word: String },
    /// Show or change configuration
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
    /// Export your words and progress (.json full backup, .csv word list)
    Export { path: PathBuf },
    /// Import words (.json backup or .csv word list with a "word" column)
    Import { path: PathBuf },
}

#[derive(Debug, Clone, Default, clap::Args)]
pub struct StudyArgs {
    /// Practice mode
    #[arg(long, value_enum, default_value_t = StudyMode::Memory)]
    pub mode: StudyMode,
    /// Stop after this many minutes
    #[arg(long, short)]
    pub minutes: Option<u32>,
    /// Stop after this many answers
    #[arg(long, short)]
    pub count: Option<usize>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum StudyMode {
    /// Flashcards: recall the meaning, then rate yourself
    #[default]
    Memory,
    /// Read the definition, type the word
    Spelling,
    /// Hear the pronunciation, type the word
    Listening,
}

#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    /// Show current configuration and data locations
    Show,
    /// Change a configuration value
    Set { key: String, value: String },
}
