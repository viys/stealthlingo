//! `export` / `import`: `.json` is a full backup, `.csv` is a plain word list.

use std::fs;
use std::path::Path;

use anyhow::{bail, Context as _, Result};
use chrono::Utc;

use super::{cached_or_fetch, Context};
use crate::storage::{AddOutcome, Backup, ImportSummary};

enum Format {
    Json,
    Csv,
}

fn format_of(path: &Path) -> Result<Format> {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("json") => Ok(Format::Json),
        Some("csv") => Ok(Format::Csv),
        _ => bail!("unsupported file type; use a .json (full backup) or .csv (word list) path"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportResult {
    Json {
        saved: usize,
        cached: usize,
        attempts: usize,
    },
    Csv {
        words: usize,
    },
}

impl ExportResult {
    pub fn summary(&self, path: &Path) -> String {
        match self {
            Self::Json {
                saved,
                cached,
                attempts,
            } => format!(
                "Exported {saved} saved words, {cached} cached entries and {attempts} answers to {}.",
                path.display()
            ),
            Self::Csv { words } => format!("Exported {words} words to {}.", path.display()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportResult {
    Json(ImportSummary),
    /// Per-line `details` are for the CLI; the full-screen UI only shows the counts.
    Csv(CsvImport),
}

impl ImportResult {
    pub fn summary(&self) -> String {
        match self {
            Self::Json(summary) => format!(
                "Imported: {} new words, {} refreshed entries, {} new and {} updated progress records, {} answers.",
                summary.words_added,
                summary.words_updated,
                summary.progress_added,
                summary.progress_updated,
                summary.attempts_added
            ),
            Self::Csv(CsvImport {
                added,
                existing,
                failed,
                ..
            }) => format!(
                "Imported {added} new words ({existing} already saved, {failed} failed)."
            ),
        }
    }
}

pub fn export(ctx: &Context, path: &Path) -> Result<ExportResult> {
    match format_of(path)? {
        Format::Json => {
            let backup = ctx.db.export_backup(Utc::now())?;
            let text = serde_json::to_string_pretty(&backup)?;
            fs::write(path, text + "\n")
                .with_context(|| format!("could not write {}", path.display()))?;
            let saved = backup.words.iter().filter(|w| w.progress.is_some()).count();
            let attempts: usize = backup.words.iter().map(|w| w.attempts.len()).sum();
            Ok(ExportResult::Json {
                saved,
                cached: backup.words.len(),
                attempts,
            })
        }
        Format::Csv => {
            let words = ctx.db.list_words(None)?;
            let mut writer = csv::Writer::from_path(path)
                .with_context(|| format!("could not write {}", path.display()))?;
            writer.write_record(["word", "note", "status", "due_at", "definition"])?;
            for w in &words {
                writer.write_record([
                    w.display_word.as_str(),
                    w.note.as_deref().unwrap_or(""),
                    w.status.as_str(),
                    &crate::time::to_db(w.due_at),
                    &w.summary,
                ])?;
            }
            writer.flush()?;
            Ok(ExportResult::Csv { words: words.len() })
        }
    }
}

pub fn import(ctx: &mut Context, path: &Path) -> Result<ImportResult> {
    match format_of(path)? {
        Format::Json => {
            let text = fs::read_to_string(path)
                .with_context(|| format!("could not read {}", path.display()))?;
            let backup: Backup = serde_json::from_str(text.trim_start_matches('\u{feff}'))
                .with_context(|| {
                    format!("{} is not a valid StealthLingo backup", path.display())
                })?;
            Ok(ImportResult::Json(ctx.db.import_backup(&backup)?))
        }
        Format::Csv => Ok(ImportResult::Csv(import_csv(ctx, path)?)),
    }
}

/// CLI entry: writes the file and prints the same messages as before.
pub fn export_cmd(ctx: &Context, path: &Path) -> Result<()> {
    println!("{}", export(ctx, path)?.summary(path));
    Ok(())
}

/// CLI entry: merges the file and prints per-line CSV notes plus a summary.
pub fn import_cmd(ctx: &mut Context, path: &Path) -> Result<()> {
    let result = import(ctx, path)?;
    if let ImportResult::Csv(CsvImport { details, .. }) = &result {
        for line in details {
            println!("{line}");
        }
    }
    println!("{}", result.summary());
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvImport {
    pub added: usize,
    pub existing: usize,
    pub failed: usize,
    pub details: Vec<String>,
}

/// Reads a CSV with a `word` column (or uses the first column) and an optional
/// `note` column, looking up and saving each word.
fn import_csv(ctx: &Context, path: &Path) -> Result<CsvImport> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_path(path)
        .with_context(|| format!("could not read {}", path.display()))?;
    let headers = reader.headers()?.clone();
    let column = |name: &str| headers.iter().position(|h| h.eq_ignore_ascii_case(name));
    let word_col = column("word").unwrap_or(0);
    let note_col = column("note");

    let mut out = CsvImport {
        added: 0,
        existing: 0,
        failed: 0,
        details: Vec::new(),
    };
    for (index, record) in reader.records().enumerate() {
        let line = index + 2;
        let record = match record {
            Ok(record) => record,
            Err(err) => {
                out.details.push(format!("  line {line}: skipped ({err})"));
                out.failed += 1;
                continue;
            }
        };
        let Some(word) = record.get(word_col).filter(|w| !w.is_empty()) else {
            continue;
        };
        let note = note_col
            .and_then(|c| record.get(c))
            .filter(|n| !n.is_empty());
        let result = cached_or_fetch(ctx, word)
            .and_then(|(cached, _)| ctx.db.add_to_collection(cached.id, note, Utc::now()));
        match result {
            Ok(AddOutcome::Added | AddOutcome::Restored) => {
                out.details.push(format!("  + {word}"));
                out.added += 1;
            }
            Ok(_) => out.existing += 1,
            Err(err) => {
                out.details
                    .push(format!("  line {line}: could not add \"{word}\": {err:#}"));
                out.failed += 1;
            }
        }
    }
    Ok(out)
}
