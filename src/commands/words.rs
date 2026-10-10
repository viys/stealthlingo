use anyhow::Result;
use chrono::Utc;

use super::{require_word, truncate, Context};
use crate::storage::WordFilter;
use crate::time::describe_due;

pub fn list(ctx: &Context, query: Option<&str>, filter: &WordFilter) -> Result<()> {
    let now = Utc::now();
    let words: Vec<_> = ctx
        .db
        .list_words(query)?
        .into_iter()
        .filter(|w| filter.matches(w, now))
        .collect();
    let conditions = match (query, filter.is_empty()) {
        (Some(q), true) => format!("matching \"{q}\""),
        (Some(q), false) => format!("matching \"{q}\" ({})", filter.describe()),
        (None, false) => format!("matching {}", filter.describe()),
        (None, true) => String::new(),
    };
    if words.is_empty() {
        if conditions.is_empty() {
            println!("Your word list is empty. Save one with `stealthlingo add <word>`.");
        } else {
            println!("No saved words {conditions}.");
        }
        return Ok(());
    }

    let width = words
        .iter()
        .map(|w| w.display_word.chars().count())
        .max()
        .unwrap_or(4)
        .clamp(4, 24);
    println!(
        "{:<width$}  {:<8}  {:<6}  {:<5}  NOTE / DEFINITION",
        "WORD", "STATUS", "DUE", "BY"
    );
    for w in &words {
        let due = if w.status == crate::learning::Status::New {
            "new".to_string()
        } else {
            describe_due(w.due_at, now)
        };
        let detail = match &w.note {
            Some(note) => format!("{note} | {}", w.summary),
            None => w.summary.clone(),
        };
        println!(
            "{:<width$}  {:<8}  {:<6}  {:<5}  {}",
            truncate(&w.display_word, width),
            w.status.as_str(),
            due,
            if w.added_by_agent { "agent" } else { "you" },
            truncate(&detail, 60)
        );
    }
    println!();
    let noun = if words.len() == 1 { "word" } else { "words" };
    if conditions.is_empty() {
        println!("{} saved {noun}.", words.len());
    } else {
        println!("{} {noun} {conditions}.", words.len());
    }
    Ok(())
}

pub fn remove(ctx: &Context, word: &str) -> Result<()> {
    let headword = require_word(word)?;
    if ctx.db.remove_from_collection(&headword, Utc::now())? {
        println!("Removed \"{}\" from your word list.", word.trim());
    } else {
        println!("\"{}\" is not in your word list.", word.trim());
    }
    Ok(())
}
