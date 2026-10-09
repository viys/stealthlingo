use anyhow::Result;
use chrono::Utc;

use super::{require_word, truncate, Context};
use crate::time::describe_due;

pub fn list(ctx: &Context, query: Option<&str>) -> Result<()> {
    let words = ctx.db.list_words(query)?;
    if words.is_empty() {
        match query {
            Some(q) => println!("No saved words match \"{q}\"."),
            None => println!("Your word list is empty. Save one with `stealthlingo add <word>`."),
        }
        return Ok(());
    }

    let now = Utc::now();
    let width = words
        .iter()
        .map(|w| w.display_word.chars().count())
        .max()
        .unwrap_or(4)
        .clamp(4, 24);
    println!(
        "{:<width$}  {:<8}  {:<6}  NOTE / DEFINITION",
        "WORD", "STATUS", "DUE"
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
            "{:<width$}  {:<8}  {:<6}  {}",
            truncate(&w.display_word, width),
            w.status.as_str(),
            due,
            truncate(&detail, 60)
        );
    }
    println!();
    let noun = if words.len() == 1 { "word" } else { "words" };
    match query {
        Some(q) => println!("{} {noun} matching \"{q}\".", words.len()),
        None => println!("{} saved {noun}.", words.len()),
    }
    Ok(())
}

pub fn remove(ctx: &Context, word: &str) -> Result<()> {
    let headword = require_word(word)?;
    if ctx.db.remove_from_collection(&headword)? {
        println!("Removed \"{}\" from your word list.", word.trim());
    } else {
        println!("\"{}\" is not in your word list.", word.trim());
    }
    Ok(())
}
