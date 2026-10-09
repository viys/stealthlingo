pub mod add;
pub mod audio;
pub mod config;
pub mod io;
pub mod links;
pub mod lookup;
pub mod review;
pub mod stats;
pub mod study;
pub mod words;

use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use chrono::Utc;

use crate::audio::AudioPlayer;
use crate::cli::{Cli, Command, LinksAction, StudyArgs};
use crate::config::{Config, Paths};
use crate::dictionary::{self, normalize_headword, DictionaryClient, Endpoints};
use crate::error::DictionaryError;
use crate::storage::{CachedWord, Database};

/// Everything a command needs: resolved paths, configuration and the database.
pub struct Context {
    pub paths: Paths,
    pub config: Config,
    pub db: Database,
    pub endpoints: Endpoints,
}

impl Context {
    pub fn load() -> Result<Self> {
        Self::load_from(Paths::resolve()?)
    }

    pub fn load_from(paths: Paths) -> Result<Self> {
        paths.ensure_exists()?;
        let config = Config::load(&paths.config_file())?;
        let db = Database::open(&paths.database())?;
        Ok(Self {
            paths,
            config,
            db,
            endpoints: Endpoints::default(),
        })
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(self.config.http_timeout_secs.max(1))
    }

    pub fn dictionary(&self) -> Result<DictionaryClient> {
        Ok(DictionaryClient::new(
            self.endpoints.clone(),
            self.timeout(),
        )?)
    }

    pub fn audio_player(&self) -> Result<AudioPlayer> {
        AudioPlayer::new(self.paths.audio_dir(), self.timeout())
    }
}

pub fn run(cli: Cli) -> Result<()> {
    crate::learning::session::install_interrupt_handler();
    let mut ctx = Context::load()?;
    match cli.command {
        None => default_entry(&mut ctx),
        Some(Command::Lookup { word, all }) => {
            let detail = if all {
                lookup::Detail::Full
            } else {
                lookup::Detail::Brief
            };
            lookup::run(&ctx, &word, detail)
        }
        Some(Command::Add { word, note }) => add::run(&ctx, &word, note.as_deref()),
        Some(Command::Remove { word }) => words::remove(&ctx, &word),
        Some(Command::Words) => words::list(&ctx, None),
        Some(Command::Search { query }) => words::list(&ctx, Some(&query)),
        Some(Command::Study(args)) => study::run(&mut ctx, &args),
        Some(Command::Review { count, minutes }) => review::run(&mut ctx, count, minutes),
        Some(Command::Stats) => stats::run(&ctx),
        Some(Command::Audio { word }) => audio::run(&ctx, &word),
        Some(Command::Config { action }) => config::run(&mut ctx, action),
        Some(Command::Links { action }) => match action.unwrap_or(LinksAction::Status) {
            LinksAction::Status => links::status(),
            LinksAction::Install => links::install(),
            LinksAction::Uninstall => links::uninstall(),
            LinksAction::Open { link } => links::open(&ctx, &link),
        },
        Some(Command::Export { path }) => io::export(&ctx, &path),
        Some(Command::Import { path }) => io::import(&mut ctx, &path),
    }
}

/// `stealthlingo` with no command: review if anything is due, otherwise study,
/// or explain how to get started with an empty word list.
fn default_entry(ctx: &mut Context) -> Result<()> {
    if ctx.db.collection_size()? == 0 {
        println!("Welcome to StealthLingo! Your word list is empty.");
        println!();
        println!("  stealthlingo lookup ephemeral   look a word up");
        println!("  stealthlingo add ephemeral      save it for practice");
        println!("  stealthlingo                    start today's practice");
        println!();
        println!("Run `stealthlingo --help` for all commands.");
        return Ok(());
    }
    if ctx.db.count_due(Utc::now())? > 0 {
        review::run(ctx, None, None)
    } else {
        study::run(ctx, &StudyArgs::default())
    }
}

fn require_word(word: &str) -> Result<String> {
    let headword = normalize_headword(word);
    if headword.is_empty() {
        bail!("please provide a word");
    }
    Ok(headword)
}

/// How fresh the data returned by [`fetch_word`] is.
pub enum Freshness {
    Fresh,
    Cached,
    /// The network request failed; showing the cached copy instead.
    Fallback(DictionaryError),
}

fn offline_error(word: &str, err: DictionaryError) -> anyhow::Error {
    if err.is_service_side() {
        anyhow!(
            "{err}\n\"{word}\" is not in your local cache yet, so Wiktionary cannot look it up right now; \
             try again in a few minutes. Words you have already saved work offline."
        )
    } else if err.is_transient() {
        anyhow!("{err}\n\"{word}\" is not in your local cache yet, so looking it up needs a working connection.")
    } else {
        anyhow!(err)
    }
}

/// Queries Wiktionary and refreshes the cache, falling back to cached data
/// when the service is unreachable.
pub fn fetch_word(ctx: &Context, word: &str) -> Result<(CachedWord, Freshness)> {
    let headword = require_word(word)?;
    match ctx.dictionary()?.fetch(word) {
        Ok(fetched) => {
            let key = Some(normalize_headword(&fetched.entry.word))
                .filter(|k| !k.is_empty())
                .unwrap_or_else(|| headword.clone());
            let source = if fetched.complete {
                dictionary::SOURCE
            } else {
                // Keep a complete copy rather than replace it with one that
                // lacks pronunciations and audio.
                if let Some(cached) = ctx
                    .db
                    .find_cached(&key)?
                    .filter(|c| c.source == dictionary::SOURCE)
                {
                    ctx.db.add_alias(&headword, cached.id)?;
                    return Ok((cached, Freshness::Cached));
                }
                dictionary::PARTIAL_SOURCE
            };
            let id =
                ctx.db
                    .cache_word(&key, &fetched.entry, &fetched.raw_json, source, Utc::now())?;
            // The dictionary may answer with another headword ("ran" -> "run").
            ctx.db.add_alias(&headword, id)?;
            let cached = ctx
                .db
                .find_cached(&key)?
                .ok_or_else(|| anyhow!("the dictionary entry was not saved"))?;
            Ok((cached, Freshness::Fresh))
        }
        Err(err) if err.is_transient() => match ctx.db.find_cached(&headword)? {
            Some(cached) => Ok((cached, Freshness::Fallback(err))),
            None => Err(offline_error(word.trim(), err)),
        },
        Err(err) => Err(err.into()),
    }
}

/// Returns cached data when available and only goes to the network otherwise.
/// Entries from a dictionary used by older versions, or cached without their
/// pronunciation data, are refreshed from Wiktionary; the old copy is kept if
/// that fails. When Wiktionary does not have the word at all, the old copy is
/// marked as current so it is not looked up again every time.
pub fn cached_or_fetch(ctx: &Context, word: &str) -> Result<(CachedWord, Freshness)> {
    let headword = require_word(word)?;
    if let Some(cached) = ctx.db.find_cached(&headword)? {
        if cached.source == dictionary::SOURCE {
            return Ok((cached, Freshness::Cached));
        }
        return match fetch_word(ctx, word) {
            Ok(fresh) => Ok(fresh),
            Err(err) => {
                if let Some(DictionaryError::NotFound(_)) = err.downcast_ref() {
                    ctx.db.set_source(cached.id, dictionary::SOURCE)?;
                }
                Ok((cached, Freshness::Cached))
            }
        };
    }
    fetch_word(ctx, word)
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        let mut out: String = text.chars().take(max_chars.saturating_sub(3)).collect();
        out.push_str("...");
        out
    }
}

fn percent(part: i64, whole: i64) -> String {
    if whole == 0 {
        "-".to_string()
    } else {
        format!("{:.0}%", part as f64 * 100.0 / whole as f64)
    }
}
