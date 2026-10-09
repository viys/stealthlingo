//! `export` / `import`: `.json` is a full backup, `.csv` is a plain word list.

use std::fs;
use std::path::Path;

use anyhow::{bail, Context as _, Result};
use chrono::Utc;

use super::{cached_or_fetch, Context};
use crate::storage::{AddOutcome, Backup};

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

pub fn export(ctx: &Context, path: &Path) -> Result<()> {
    match format_of(path)? {
        Format::Json => {
            let backup = ctx.db.export_backup(Utc::now())?;
            let text = serde_json::to_string_pretty(&backup)?;
            fs::write(path, text + "\n")
                .with_context(|| format!("could not write {}", path.display()))?;
            let saved = backup.words.iter().filter(|w| w.progress.is_some()).count();
            let attempts: usize = backup.words.iter().map(|w| w.attempts.len()).sum();
            println!(
                "Exported {saved} saved words, {} cached entries and {attempts} answers to {}.",
                backup.words.len(),
                path.display()
            );
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
            println!("Exported {} words to {}.", words.len(), path.display());
        }
    }
    Ok(())
}

pub fn import(ctx: &mut Context, path: &Path) -> Result<()> {
    match format_of(path)? {
        Format::Json => {
            let text = fs::read_to_string(path)
                .with_context(|| format!("could not read {}", path.display()))?;
            let backup: Backup = serde_json::from_str(text.trim_start_matches('\u{feff}'))
                .with_context(|| {
                    format!("{} is not a valid StealthLingo backup", path.display())
                })?;
            let summary = ctx.db.import_backup(&backup)?;
            println!(
                "Imported: {} new words, {} refreshed entries, {} new and {} updated progress records, {} answers.",
                summary.words_added,
                summary.words_updated,
                summary.progress_added,
                summary.progress_updated,
                summary.attempts_added
            );
        }
        Format::Csv => import_csv(ctx, path)?,
    }
    Ok(())
}

/// Reads a CSV with a `word` column (or uses the first column) and an optional
/// `note` column, looking up and saving each word.
fn import_csv(ctx: &Context, path: &Path) -> Result<()> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_path(path)
        .with_context(|| format!("could not read {}", path.display()))?;
    let headers = reader.headers()?.clone();
    let column = |name: &str| headers.iter().position(|h| h.eq_ignore_ascii_case(name));
    let word_col = column("word").unwrap_or(0);
    let note_col = column("note");

    let (mut added, mut existing, mut failed) = (0, 0, 0);
    for (index, record) in reader.records().enumerate() {
        let line = index + 2;
        let record = match record {
            Ok(record) => record,
            Err(err) => {
                println!("  line {line}: skipped ({err})");
                failed += 1;
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
            Ok(AddOutcome::Added) => {
                println!("  + {word}");
                added += 1;
            }
            Ok(_) => existing += 1,
            Err(err) => {
                println!("  line {line}: could not add \"{word}\": {err:#}");
                failed += 1;
            }
        }
    }
    println!("Imported {added} new words ({existing} already saved, {failed} failed).");
    Ok(())
}
