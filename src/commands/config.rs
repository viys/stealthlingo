use anyhow::Result;

use super::Context;
use crate::cli::ConfigAction;
use crate::dictionary::{merriam_webster, Source};
use crate::storage::Database;

pub fn run(ctx: &mut Context, action: Option<ConfigAction>) -> Result<()> {
    match action.unwrap_or(ConfigAction::Show) {
        ConfigAction::Show => {
            for (key, value) in ctx.config.entries() {
                println!("{key:<20} = {value}");
            }
            println!();
            println!("Dictionary sources: {}", Source::names());
            println!();
            println!("Data directory:   {}", ctx.paths.data_dir.display());
            println!("Database:         {}", ctx.paths.database().display());
            println!("Config file:      {}", ctx.paths.config_file().display());
            println!("Audio cache:      {}", ctx.paths.audio_dir().display());
            println!(
                "Schema version:   {} (latest {})",
                ctx.db.schema_version()?,
                Database::latest_schema_version()
            );
        }
        ConfigAction::Set { key, value } => {
            let key = ctx.config.set(&key, &value)?;
            ctx.config.save(&ctx.paths.config_file())?;
            if let Some((_, shown)) = ctx.config.entries().into_iter().find(|(k, _)| *k == key) {
                println!("{key} = {shown}");
            }
            if ctx.config.dictionary_source == Source::MerriamWebster
                && ctx.config.merriam_webster_key.is_none()
            {
                println!("Note: {}", merriam_webster::missing_key_error());
            }
        }
    }
    Ok(())
}
