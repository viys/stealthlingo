use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::learning::Status;
use crate::storage::WordFilter;

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
    Words {
        #[command(flatten)]
        filter: WordFilterArgs,
    },
    /// Search saved words and personal notes
    Search {
        query: String,
        #[command(flatten)]
        filter: WordFilterArgs,
    },
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
    /// Serve AI agents over MCP on stdin/stdout (started by the agent's client)
    Mcp,
}

#[derive(Debug, Clone, Default, clap::Args)]
pub struct WordFilterArgs {
    /// Only words at this stage of learning
    #[arg(long, value_enum)]
    pub status: Option<StatusArg>,
    /// Only words with this part of speech; the start is enough (adj, n, v)
    #[arg(long)]
    pub pos: Option<String>,
    /// Only words due for review now
    #[arg(long)]
    pub due: bool,
}

impl WordFilterArgs {
    pub fn filter(&self) -> WordFilter {
        WordFilter {
            status: self.status.map(StatusArg::status),
            part_of_speech: self
                .pos
                .as_deref()
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(str::to_string),
            due: self.due,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum StatusArg {
    New,
    Learning,
    Review,
    Mastered,
}

impl StatusArg {
    pub fn status(self) -> Status {
        match self {
            Self::New => Status::New,
            Self::Learning => Status::Learning,
            Self::Review => Status::Review,
            Self::Mastered => Status::Mastered,
        }
    }
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
    /// Practise words you got wrong recently, most recent mistakes first,
    /// whether or not they are due
    #[arg(long)]
    pub mistakes: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum StudyMode {
    /// Flashcards: recall the meaning, then rate yourself
    #[default]
    Memory,
    /// Read the definition, type the word
    Spelling,
    /// Read the definition and part of the word (e _ h e _ e r _ l), type the word
    Letters,
    /// Hear the pronunciation, type the word
    Listening,
    /// A mix of the other modes, chosen per word
    Mixed,
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
    /// Restore one setting, or all of them, to the default
    Reset {
        /// Setting to restore; without it, every setting
        key: Option<String>,
        /// Reset everything without asking for confirmation
        #[arg(long, short)]
        yes: bool,
    },
}
