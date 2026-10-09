//! Simplified SM-2 spaced repetition. Pure functions only: no I/O, no clock.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

pub const DEFAULT_EASE: f64 = 2.5;
pub const MIN_EASE: f64 = 1.3;
pub const MAX_EASE: f64 = 3.0;
pub const RELEARN_DELAY_MINUTES: i64 = 10;
pub const MASTERED_INTERVAL_DAYS: f64 = 21.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Grade {
    Again,
    Hard,
    Good,
    Easy,
}

impl Grade {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Again => "again",
            Self::Hard => "hard",
            Self::Good => "good",
            Self::Easy => "easy",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "again" => Some(Self::Again),
            "hard" => Some(Self::Hard),
            "good" => Some(Self::Good),
            "easy" => Some(Self::Easy),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    New,
    Learning,
    Review,
    Mastered,
}

impl Status {
    pub const ALL: [Status; 4] = [Self::New, Self::Learning, Self::Review, Self::Mastered];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Learning => "learning",
            Self::Review => "review",
            Self::Mastered => "mastered",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.as_str() == value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScheduleState {
    pub status: Status,
    pub repetitions: u32,
    pub interval_days: f64,
    pub ease_factor: f64,
    pub lapses: u32,
    pub due_at: DateTime<Utc>,
    pub last_reviewed_at: Option<DateTime<Utc>>,
}

impl ScheduleState {
    /// State of a freshly saved word: due immediately.
    pub fn new(now: DateTime<Utc>) -> Self {
        Self {
            status: Status::New,
            repetitions: 0,
            interval_days: 0.0,
            ease_factor: DEFAULT_EASE,
            lapses: 0,
            due_at: now,
            last_reviewed_at: None,
        }
    }

    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        self.due_at <= now
    }
}

/// Interval a successful "Good" answer would produce.
fn good_interval(repetitions: u32, previous: f64, ease: f64) -> f64 {
    match repetitions {
        1 => 1.0,
        2 => 3.0,
        _ => (previous * ease).max(previous + 1.0),
    }
}

/// Computes the next schedule after answering with `grade` at `now`.
pub fn schedule(state: &ScheduleState, grade: Grade, now: DateTime<Utc>) -> ScheduleState {
    let mut next = state.clone();
    next.last_reviewed_at = Some(now);

    if grade == Grade::Again {
        if state.status != Status::New {
            next.lapses += 1;
        }
        next.repetitions = 0;
        next.interval_days = 0.0;
        next.ease_factor = (state.ease_factor - 0.2).max(MIN_EASE);
        next.status = Status::Learning;
        next.due_at = now + Duration::minutes(RELEARN_DELAY_MINUTES);
        return next;
    }

    next.repetitions = state.repetitions + 1;
    let previous = state.interval_days.max(0.0);
    let (interval, ease) = match grade {
        Grade::Hard => ((previous * 1.2).max(0.5), state.ease_factor - 0.15),
        Grade::Good => (
            good_interval(next.repetitions, previous, state.ease_factor),
            state.ease_factor,
        ),
        Grade::Easy => {
            let good = good_interval(next.repetitions, previous, state.ease_factor);
            ((good * 1.3).max(good + 1.0), state.ease_factor + 0.15)
        }
        Grade::Again => unreachable!("handled above"),
    };
    next.interval_days = (interval * 100.0).round() / 100.0;
    next.ease_factor = ease.clamp(MIN_EASE, MAX_EASE);
    next.due_at = now + Duration::seconds((next.interval_days * 86_400.0).round() as i64);
    next.status = if next.interval_days >= MASTERED_INTERVAL_DAYS {
        Status::Mastered
    } else if next.repetitions >= 2 {
        Status::Review
    } else {
        Status::Learning
    };
    next
}
