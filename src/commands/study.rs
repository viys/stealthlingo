use std::time::Duration;

use anyhow::{bail, Result};
use chrono::Utc;

use super::{percent, Context};
use crate::cli::{StudyArgs, StudyMode};
use crate::learning::session::{build_queue, run_session, SessionPlan};
use crate::learning::Status;
use crate::time::{describe_due, local_day_start};

pub fn run(ctx: &mut Context, args: &StudyArgs) -> Result<()> {
    start_session(ctx, args.mode, false, args.minutes, args.count)
}

/// Shared by `study`, `review` and the default command.
pub fn start_session(
    ctx: &mut Context,
    mode: StudyMode,
    due_only: bool,
    minutes: Option<u32>,
    count: Option<usize>,
) -> Result<()> {
    if minutes == Some(0) {
        bail!("--minutes must be at least 1");
    }
    if count == Some(0) {
        bail!("--count must be at least 1");
    }

    let now = Utc::now();
    let queue = build_queue(
        &ctx.db,
        mode,
        due_only,
        ctx.config.daily_new_limit,
        now,
        local_day_start(now),
    )?;
    if queue.is_empty() {
        explain_empty_queue(ctx, mode, due_only)?;
        return Ok(());
    }

    // Without explicit limits a session uses the configured default length.
    let minutes = match (minutes, count) {
        (None, None) => Some(ctx.config.default_minutes.max(1)),
        _ => minutes,
    };
    let plan = SessionPlan {
        mode,
        time_limit: minutes.map(|m| Duration::from_secs(u64::from(m) * 60)),
        max_answers: count,
    };

    let due = queue
        .iter()
        .filter(|i| i.schedule.status != Status::New)
        .count();
    let fresh = queue.len() - due;
    println!(
        "StealthLingo · {}",
        if due_only { "Review" } else { "Quick Study" }
    );
    let mut limits = Vec::new();
    if let Some(m) = minutes {
        limits.push(format!("{m} min"));
    }
    if let Some(c) = count {
        limits.push(format!("{c} answers"));
    }
    println!(
        "Mode: {} · Session: {} · Due: {due} · New: {fresh}",
        mode.label(),
        limits.join(", ")
    );

    let audio = ctx.audio_player()?;
    let summary = run_session(&mut ctx.db, &audio, &plan, queue)?;

    println!();
    if summary.answered == 0 {
        println!("No answers recorded.");
    } else {
        println!(
            "Session saved: {} answered, {} correct ({}){}.",
            summary.answered,
            summary.correct,
            percent(summary.correct as i64, summary.answered as i64),
            if summary.skipped > 0 {
                format!(", {} skipped", summary.skipped)
            } else {
                String::new()
            }
        );
    }
    let now = Utc::now();
    let due_now = ctx.db.count_due(now)?;
    if due_now > 0 {
        let noun = if due_now == 1 { "word is" } else { "words are" };
        println!("{due_now} {noun} due. Run `stealthlingo review` to continue.");
    } else if let Some(next) = ctx.db.next_due_at()? {
        println!("Next review {}.", describe_due(next, now));
    }
    Ok(())
}

fn explain_empty_queue(ctx: &Context, mode: StudyMode, due_only: bool) -> Result<()> {
    let now = Utc::now();
    if ctx.db.collection_size()? == 0 {
        println!("Your word list is empty. Save a word first: `stealthlingo add <word>`.");
        return Ok(());
    }
    if mode == StudyMode::Listening {
        println!("No saved words with pronunciation audio are ready for listening practice.");
    } else if due_only {
        println!("Nothing is due for review.");
    } else {
        println!("Nothing to study right now.");
        if ctx.db.count_new_ready()? > 0 {
            println!(
                "You reached today's limit of {} new words (change it with `stealthlingo config set daily_new_limit <n>`).",
                ctx.config.daily_new_limit
            );
        }
    }
    if let Some(next) = ctx.db.next_due_at()? {
        println!("Next review {}.", describe_due(next, now));
    }
    Ok(())
}
