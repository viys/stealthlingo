use anyhow::{bail, Result};

use super::{cached_or_fetch, Context};
use crate::dictionary::normalize_headword;

pub fn run(ctx: &Context, word: &str) -> Result<()> {
    let (cached, _) = cached_or_fetch(ctx, word)?;
    let Some(url) = cached.entry.audio_url() else {
        bail!(
            "the dictionary has no pronunciation audio for \"{}\"",
            cached.entry.word
        );
    };
    let accent = cached
        .entry
        .audio()
        .and_then(|p| p.accent.as_deref())
        .map(|a| format!(" ({a} recording)"))
        .unwrap_or_default();
    println!("Playing \"{}\"{accent}", cached.entry.word);
    ctx.audio_player()?
        .play(&normalize_headword(&cached.entry.word), url)
}
