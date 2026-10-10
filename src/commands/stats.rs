use anyhow::Result;
use chrono::{DateTime, Utc};

use super::{percent, Context};
use crate::time::{
    describe_due, format_local, local_day_start, local_day_start_before, local_weekday,
};

/// "6 / 10 words · 4 to go", or "10 / 10 words · reached".
pub fn goal_progress(done: i64, goal: u32) -> String {
    let goal = i64::from(goal);
    if done >= goal {
        format!("{done} / {goal} words · reached")
    } else {
        format!("{done} / {goal} words · {} to go", goal - done)
    }
}

/// Different words answered on each of the last `days` local days, oldest
/// first, with the day's start.
pub fn recent_days(
    ctx: &Context,
    now: DateTime<Utc>,
    days: u32,
) -> Result<Vec<(DateTime<Utc>, i64)>> {
    (0..days)
        .rev()
        .map(|ago| {
            let start = local_day_start_before(now, ago);
            let end = match ago {
                0 => now.max(start) + chrono::Duration::days(1),
                _ => local_day_start_before(now, ago - 1),
            };
            Ok((start, ctx.db.count_words_practised(start, end)?))
        })
        .collect()
}

/// `Sat 12 ✓ · Sun 4 · ...` for the last seven days.
pub fn week_line(days: &[(DateTime<Utc>, i64)], goal: u32) -> String {
    days.iter()
        .map(|(start, n)| {
            let mark = if *n >= i64::from(goal) { " ✓" } else { "" };
            format!("{} {n}{mark}", local_weekday(*start))
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

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
    let goal = ctx.config.daily_goal;
    if goal > 0 {
        println!("Daily goal:   {}", goal_progress(stats.words_today, goal));
        let days = recent_days(ctx, now, 7)?;
        let reached = days.iter().filter(|(_, n)| *n >= i64::from(goal)).count();
        println!(
            "Last 7 days:  {}  ({reached} of 7 reached)",
            week_line(&days, goal)
        );
    }
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
