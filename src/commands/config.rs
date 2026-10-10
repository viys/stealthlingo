use std::io::{self, BufRead, IsTerminal, Write};

use anyhow::{bail, Result};

use super::Context;
use crate::cli::ConfigAction;
use crate::config::{Config, SETTINGS};
use crate::storage::Database;

pub fn run(ctx: &mut Context, action: Option<ConfigAction>) -> Result<()> {
    match action.unwrap_or(ConfigAction::Show) {
        ConfigAction::Show => show(ctx)?,
        ConfigAction::Set { key, value } => {
            let key = ctx.config.set(&key, &value)?;
            ctx.save_config()?;
            print_value(&ctx.config, key);
        }
        ConfigAction::Reset { key: Some(key), .. } => {
            let key = ctx.config.reset(&key)?;
            ctx.save_config()?;
            print_value(&ctx.config, key);
        }
        ConfigAction::Reset { key: None, yes } => {
            if !yes && !confirm("Reset all settings to their defaults? [y/N] ")? {
                println!("Nothing changed.");
                return Ok(());
            }
            ctx.config = Config::default();
            ctx.save_config()?;
            println!("All settings are back to their defaults.");
        }
    }
    Ok(())
}

fn print_value(config: &Config, key: &str) {
    if let Some((_, shown)) = config.entries().into_iter().find(|(k, _)| *k == key) {
        println!("{key} = {shown}");
    }
}

fn confirm(prompt: &str) -> Result<bool> {
    if !io::stdin().is_terminal() {
        bail!("refusing to reset every setting without confirmation; run `stealthlingo config reset --yes`");
    }
    print!("{prompt}");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    Ok(matches!(line.trim().to_lowercase().as_str(), "y" | "yes"))
}

fn show(ctx: &Context) -> Result<()> {
    let value_width = SETTINGS
        .iter()
        .map(|s| s.value(&ctx.config).len())
        .max()
        .unwrap_or(0);
    for setting in SETTINGS {
        println!(
            "{:<20} = {:<value_width$}   default {:<4} {}",
            setting.key,
            setting.value(&ctx.config),
            setting.default_value(),
            setting.range()
        );
    }
    println!();
    println!("Dictionary:       English Wiktionary");
    println!("Data directory:   {}", ctx.paths.data_dir.display());
    println!("Database:         {}", ctx.paths.database().display());
    println!("Config file:      {}", ctx.paths.config_file().display());
    println!("Audio cache:      {}", ctx.paths.audio_dir().display());
    println!(
        "Schema version:   {} (latest {})",
        ctx.db.schema_version()?,
        Database::latest_schema_version()
    );
    Ok(())
}
