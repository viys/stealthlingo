pub mod add;
pub mod audio;
pub mod config;
pub mod io;
pub mod lookup;
pub mod review;
pub mod stats;
pub mod study;
pub mod words;

use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use chrono::Utc;

use crate::audio::AudioPlayer;
use crate::cli::{Cli, Command, StudyArgs};
use crate::config::{Config, Paths};
use crate::dictionary::{normalize_headword, DictionaryClient, Source, SourceSettings};
use crate::error::DictionaryError;
use crate::storage::{CachedWord, Database};

/// Everything a command needs: resolved paths, configuration and the database.
pub struct Context {
    pub paths: Paths,
    pub config: Config,
    pub db: Database,
    /// `--source` given on the command line for this run only.
    pub source_override: Option<Source>,
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
            source_override: None,
        })
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(self.config.http_timeout_secs.max(1))
    }

    /// The dictionary used for lookups in this run.
    pub fn source(&self) -> Source {
        self.source_override
            .unwrap_or(self.config.dictionary_source)
    }

    pub fn dictionary(&self) -> Result<DictionaryClient> {
        let settings = SourceSettings {
            source: self.source(),
            free_dictionary_url: self.config.free_dictionary_url.clone(),
            merriam_webster_key: self.config.merriam_webster_key.clone(),
        };
        Ok(DictionaryClient::new(settings, self.timeout())?)
    }

    pub fn audio_player(&self) -> Result<AudioPlayer> {
        AudioPlayer::new(self.paths.audio_dir(), self.timeout())
    }
}

pub fn run(cli: Cli) -> Result<()> {
    crate::learning::session::install_interrupt_handler();
    if cli.source.is_some() && matches!(cli.command, Some(Command::Config { .. })) {
        bail!("--source only applies to lookups; use `stealthlingo config set dictionary_source <SOURCE>` to change the default");
    }
    let mut ctx = Context::load()?;
    ctx.source_override = cli.source;
    match cli.command {
        None => default_entry(&mut ctx),
        Some(Command::Lookup { word }) => lookup::run(&ctx, &word),
        Some(Command::Add { word, note }) => add::run(&ctx, &word, note.as_deref()),
        Some(Command::Remove { word }) => words::remove(&ctx, &word),
        Some(Command::Words) => words::list(&ctx, None),
        Some(Command::Search { query }) => words::list(&ctx, Some(&query)),
        Some(Command::Study(args)) => study::run(&mut ctx, &args),
        Some(Command::Review { count, minutes }) => review::run(&mut ctx, count, minutes),
        Some(Command::Stats) => stats::run(&ctx),
        Some(Command::Audio { word }) => audio::run(&ctx, &word),
        Some(Command::Config { action }) => config::run(&mut ctx, action),
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

fn offline_error(word: &str, source: Source, err: DictionaryError) -> anyhow::Error {
    if err.is_service_side() {
        let alternatives: Vec<&str> = Source::ALL
            .into_iter()
            .filter(|s| *s != source)
            .map(Source::as_str)
            .collect();
        anyhow!(
            "{err}\n\"{word}\" is not in your local cache yet, so {} cannot look it up right now.\n\
             Try another dictionary: `stealthlingo lookup {word} --source {}`, or switch for good with\n\
             `stealthlingo config set dictionary_source <{}>`. Words you have already saved work offline.",
            source.label(),
            alternatives[0],
            alternatives.join("|")
        )
    } else if err.is_transient() {
        anyhow!("{err}\n\"{word}\" is not in your local cache yet, so looking it up needs a working connection.")
    } else {
        anyhow!(err)
    }
}

/// Queries the API and refreshes the cache, falling back to cached data when
/// the service is unreachable.
pub fn fetch_word(ctx: &Context, word: &str) -> Result<(CachedWord, Freshness)> {
    let headword = require_word(word)?;
    let source = ctx.source();
    match ctx.dictionary()?.fetch(word) {
        Ok(fetched) => {
            let key = Some(normalize_headword(&fetched.entry.word))
                .filter(|k| !k.is_empty())
                .unwrap_or_else(|| headword.clone());
            let id = ctx.db.cache_word(
                &key,
                &fetched.entry,
                &fetched.raw_json,
                fetched.source,
                Utc::now(),
            )?;
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
            None => Err(offline_error(word.trim(), source, err)),
        },
        Err(err) => Err(err.into()),
    }
}

/// Returns cached data when available and only goes to the network otherwise.
/// An explicit `--source` that differs from the cached copy forces a refresh.
pub fn cached_or_fetch(ctx: &Context, word: &str) -> Result<(CachedWord, Freshness)> {
    let headword = require_word(word)?;
    if let Some(cached) = ctx.db.find_cached(&headword)? {
        if ctx.source_override.is_none_or(|s| s == cached.source) {
            return Ok((cached, Freshness::Cached));
        }
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
