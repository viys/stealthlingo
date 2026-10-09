use std::time::Duration;

use anyhow::{bail, Result};
use chrono::Utc;

use super::{percent, Context};
use crate::cli::{StudyArgs, StudyMode};
use crate::learning::session::{build_queue, run_session, SessionPlan};
use crate::learning::Status;
use crate::storage::StudyItem;
use crate::time::{describe_due, local_day_start};

/// What kind of session to start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionRequest {
    pub mode: StudyMode,
    /// Only words that are due: no new words.
    pub due_only: bool,
    pub minutes: Option<u32>,
    pub count: Option<usize>,
}

impl SessionRequest {
    pub fn study(args: &StudyArgs) -> Self {
        Self {
            mode: args.mode,
            due_only: false,
            minutes: args.minutes,
            count: args.count,
        }
    }

    pub fn review(count: Option<usize>, minutes: Option<u32>) -> Self {
        Self {
            mode: StudyMode::Memory,
            due_only: true,
            minutes,
            count,
        }
    }

    pub fn title(&self) -> &'static str {
        if self.due_only {
            "Review"
        } else {
            "Quick Study"
        }
    }
}

/// A session ready to run, or the reasons there is nothing to practice.
pub enum Prepared {
    Ready {
        plan: SessionPlan,
        queue: Vec<StudyItem>,
        due: usize,
        fresh: usize,
    },
    Empty(Vec<String>),
}

pub fn prepare(ctx: &Context, request: &SessionRequest) -> Result<Prepared> {
    if request.minutes == Some(0) {
        bail!("--minutes must be at least 1");
    }
    if request.count == Some(0) {
        bail!("--count must be at least 1");
    }

    let now = Utc::now();
    let queue = build_queue(
        &ctx.db,
        request.mode,
        request.due_only,
        ctx.config.daily_new_limit,
        now,
        local_day_start(now),
    )?;
    if queue.is_empty() {
        return Ok(Prepared::Empty(empty_queue_message(
            ctx,
            request.mode,
            request.due_only,
        )?));
    }

    // Without explicit limits a session uses the configured default length.
    let minutes = match (request.minutes, request.count) {
        (None, None) => Some(ctx.config.default_minutes.max(1)),
        _ => request.minutes,
    };
    let due = queue
        .iter()
        .filter(|i| i.schedule.status != Status::New)
        .count();
    Ok(Prepared::Ready {
        plan: SessionPlan {
            mode: request.mode,
            time_limit: minutes.map(|m| Duration::from_secs(u64::from(m) * 60)),
            max_answers: request.count,
            accent: ctx.config.accent.clone(),
        },
        fresh: queue.len() - due,
        due,
        queue,
    })
}

pub fn run(ctx: &mut Context, args: &StudyArgs) -> Result<()> {
    start_session(ctx, &SessionRequest::study(args))
}

/// Line-based session shared by `study`, `review` and the default command.
pub fn start_session(ctx: &mut Context, request: &SessionRequest) -> Result<()> {
    let (plan, queue, due, fresh) = match prepare(ctx, request)? {
        Prepared::Empty(lines) => {
            for line in lines {
                println!("{line}");
            }
            return Ok(());
        }
        Prepared::Ready {
            plan,
            queue,
            due,
            fresh,
        } => (plan, queue, due, fresh),
    };

    println!("StealthLingo · {}", request.title());
    let mut limits = Vec::new();
    if let Some(limit) = plan.time_limit {
        limits.push(format!("{} min", limit.as_secs() / 60));
    }
    if let Some(c) = plan.max_answers {
        limits.push(format!("{c} answers"));
    }
    println!(
        "Mode: {} · Session: {} · Due: {due} · New: {fresh}",
        plan.mode.label(),
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
    if let Some(line) = next_review_line(ctx)? {
        println!("{line}");
    }
    Ok(())
}

/// "3 words are due. ..." or "Next review in 4h.", if anything is scheduled.
pub fn next_review_line(ctx: &Context) -> Result<Option<String>> {
    let now = Utc::now();
    let due_now = ctx.db.count_due(now)?;
    if due_now > 0 {
        let noun = if due_now == 1 { "word is" } else { "words are" };
        return Ok(Some(format!(
            "{due_now} {noun} due. Run `stealthlingo review` to continue."
        )));
    }
    Ok(ctx
        .db
        .next_due_at()?
        .map(|next| format!("Next review {}.", describe_due(next, now))))
}

fn empty_queue_message(ctx: &Context, mode: StudyMode, due_only: bool) -> Result<Vec<String>> {
    let now = Utc::now();
    if ctx.db.collection_size()? == 0 {
        return Ok(vec![
            "Your word list is empty. Save a word first: `stealthlingo add <word>`.".to_string(),
        ]);
    }
    let mut lines = Vec::new();
    if mode == StudyMode::Listening {
        lines.push(
            "No saved words with pronunciation audio are ready for listening practice.".to_string(),
        );
    } else if due_only {
        lines.push("Nothing is due for review.".to_string());
    } else {
        lines.push("Nothing to study right now.".to_string());
        if ctx.db.count_new_ready()? > 0 {
            lines.push(format!(
                "You reached today's limit of {} new words (change it with `stealthlingo config set daily_new_limit <n>`).",
                ctx.config.daily_new_limit
            ));
        }
    }
    if let Some(next) = ctx.db.next_due_at()? {
        lines.push(format!("Next review {}.", describe_due(next, now)));
    }
    Ok(lines)
}
