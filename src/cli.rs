use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// Quick, low-distraction vocabulary practice in your terminal.
///
/// Run without a command to open the full-screen app (in a terminal), or to
/// start today's practice with `--plain`.
#[derive(Debug, Parser)]
#[command(name = "stealthlingo", version, about, long_about)]
pub struct Cli {
    /// Use plain line-by-line prompts instead of the full-screen interface
    #[arg(long, global = true)]
    pub plain: bool,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Look up a word in English Wiktionary
    Lookup {
        word: String,
        /// Show every definition with examples, synonyms and the source
        #[arg(long, short)]
        all: bool,
    },
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
    Audio {
        word: String,
        /// uk or us (default: the `accent` setting)
        #[arg(long)]
        accent: Option<String>,
    },
    /// Show or change configuration
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
    /// Ctrl+click pronunciations in `lookup` to play them (Windows)
    Links {
        #[command(subcommand)]
        action: Option<LinksAction>,
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
pub enum LinksAction {
    /// Show whether links are set up and whether this terminal can show them
    Status,
    /// Register the stealthlingo:// link handler for the current user
    Install,
    /// Remove the link handler
    Uninstall,
    /// Play the recording behind a stealthlingo:// link (what a click runs)
    #[command(hide = true)]
    Open { link: String },
}

#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    /// Show current configuration and data locations
    Show,
    /// Change a configuration value
    Set { key: String, value: String },
}
