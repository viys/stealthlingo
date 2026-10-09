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
    let label = cached.entry.phonetic.as_deref().unwrap_or("");
    println!("Playing \"{}\" {label}", cached.entry.word);
    ctx.audio_player()?
        .play(&normalize_headword(&cached.entry.word), url)
}
