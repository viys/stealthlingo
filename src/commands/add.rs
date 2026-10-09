use anyhow::Result;
use chrono::Utc;

use super::{cached_or_fetch, Context};
use crate::storage::AddOutcome;

pub fn run(ctx: &Context, word: &str, note: Option<&str>) -> Result<()> {
    let (cached, _) = cached_or_fetch(ctx, word)?;
    let outcome = ctx.db.add_to_collection(cached.id, note, Utc::now())?;
    let name = &cached.entry.word;
    match outcome {
        AddOutcome::Added => {
            println!("Saved \"{name}\" — {}", cached.entry.short_summary());
            if !cached.entry.has_audio() {
                println!("(No pronunciation audio, so it will not appear in listening practice.)");
            }
        }
        AddOutcome::Restored => {
            println!("Saved \"{name}\" again, with the study progress it had before.")
        }
        AddOutcome::AlreadySaved => println!("\"{name}\" is already in your word list."),
        AddOutcome::NoteUpdated => println!("Updated the note for \"{name}\"."),
    }
    Ok(())
}
