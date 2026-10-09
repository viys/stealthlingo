use anyhow::Result;
use chrono::Utc;

use super::{percent, Context};
use crate::time::{describe_due, format_local, local_day_start};

pub fn run(ctx: &Context) -> Result<()> {
    let now = Utc::now();
    let stats = ctx.db.stats(now, local_day_start(now))?;

    let breakdown = stats
        .status_counts
        .iter()
        .map(|(status, n)| format!("{} {n}", status.as_str()))
        .collect::<Vec<_>>()
        .join(" · ");

    println!("StealthLingo · Stats");
    println!("Saved words:  {} ({breakdown})", stats.total_words);
    println!("Due now:      {}", stats.due_now);
    println!(
        "Today:        {} answers · {} correct · {} new words",
        stats.attempts_today,
        percent(stats.correct_today, stats.attempts_today),
        stats.new_words_today
    );
    println!(
        "All time:     {} answers · {} correct",
        stats.total_attempts,
        percent(stats.total_correct, stats.total_attempts)
    );
    if let Some(next) = stats.next_due {
        println!(
            "Next review:  {} ({})",
            format_local(next),
            describe_due(next, now)
        );
    }
    Ok(())
}
