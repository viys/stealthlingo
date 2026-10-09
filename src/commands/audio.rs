use anyhow::{bail, Result};

use super::{cached_or_fetch, Context};
use crate::config::parse_accent;
use crate::dictionary::normalize_headword;

pub fn run(ctx: &Context, word: &str, accent: Option<&str>) -> Result<()> {
    let wanted = match accent {
        Some(accent) => parse_accent(accent)?,
        None => ctx.config.accent.as_str(),
    };
    let (cached, _) = cached_or_fetch(ctx, word)?;
    let Some(phonetic) = cached.entry.audio_in(Some(wanted)) else {
        bail!(
            "the dictionary has no pronunciation audio for \"{}\"",
            cached.entry.word
        );
    };
    let played = phonetic.accent.as_deref();
    let label = played
        .map(|a| format!(" ({a} recording)"))
        .unwrap_or_default();
    println!("Playing \"{}\"{label}", cached.entry.word);
    if accent.is_some() && played != Some(wanted) {
        println!("(There is no {wanted} recording of this word.)");
    }
    let url = phonetic.audio_url.as_deref().unwrap_or_default();
    ctx.audio_player()?
        .play(&normalize_headword(&cached.entry.word), url)
}
