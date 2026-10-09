use chrono::{DateTime, Duration, TimeZone, Utc};
use stealthlingo::learning::scheduling::{
    schedule, Grade, ScheduleState, Status, DEFAULT_EASE, MIN_EASE, RELEARN_DELAY_MINUTES,
};

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 1, 8, 0, 0).unwrap()
}

/// Answers `grades` one after another, each exactly when the word becomes due.
fn run(grades: &[Grade]) -> ScheduleState {
    let mut state = ScheduleState::new(t0());
    for &grade in grades {
        let now = state.due_at;
        state = schedule(&state, grade, now);
    }
    state
}

#[test]
fn new_word_is_due_immediately() {
    let state = ScheduleState::new(t0());
    assert_eq!(state.status, Status::New);
    assert!(state.is_due(t0()));
    assert_eq!(state.ease_factor, DEFAULT_EASE);
}

#[test]
fn first_good_answer_schedules_one_day() {
    let state = schedule(&ScheduleState::new(t0()), Grade::Good, t0());
    assert_eq!(state.status, Status::Learning);
    assert_eq!(state.repetitions, 1);
    assert_eq!(state.interval_days, 1.0);
    assert_eq!(state.due_at, t0() + Duration::days(1));
    assert_eq!(state.last_reviewed_at, Some(t0()));
    assert_eq!(state.lapses, 0);
}

#[test]
fn again_brings_word_back_soon_and_resets_progress() {
    let reviewed = run(&[Grade::Good, Grade::Good, Grade::Good]);
    assert_eq!(reviewed.status, Status::Review);
    let now = reviewed.due_at;
    let failed = schedule(&reviewed, Grade::Again, now);
    assert_eq!(failed.status, Status::Learning);
    assert_eq!(failed.repetitions, 0);
    assert_eq!(failed.interval_days, 0.0);
    assert_eq!(failed.lapses, 1);
    assert!(failed.ease_factor < reviewed.ease_factor);
    assert_eq!(
        failed.due_at,
        now + Duration::minutes(RELEARN_DELAY_MINUTES)
    );
}

#[test]
fn again_on_brand_new_word_is_not_a_lapse() {
    let state = schedule(&ScheduleState::new(t0()), Grade::Again, t0());
    assert_eq!(state.lapses, 0);
    assert_eq!(state.status, Status::Learning);
}

#[test]
fn consecutive_good_answers_grow_intervals() {
    let mut state = ScheduleState::new(t0());
    let mut intervals = Vec::new();
    for _ in 0..5 {
        let now = state.due_at;
        state = schedule(&state, Grade::Good, now);
        intervals.push(state.interval_days);
    }
    assert_eq!(intervals[0], 1.0);
    assert_eq!(intervals[1], 3.0);
    assert!(intervals.windows(2).all(|w| w[1] > w[0]), "{intervals:?}");
    assert_eq!(state.status, Status::Mastered);
}

#[test]
fn grades_order_intervals_hard_good_easy() {
    let base = run(&[Grade::Good, Grade::Good]);
    let now = base.due_at;
    let hard = schedule(&base, Grade::Hard, now);
    let good = schedule(&base, Grade::Good, now);
    let easy = schedule(&base, Grade::Easy, now);
    assert!(hard.interval_days < good.interval_days);
    assert!(good.interval_days < easy.interval_days);
    assert!(hard.ease_factor < good.ease_factor);
    assert!(easy.ease_factor > good.ease_factor);
}

#[test]
fn hard_on_first_answer_is_shorter_than_a_day() {
    let state = schedule(&ScheduleState::new(t0()), Grade::Hard, t0());
    assert_eq!(state.interval_days, 0.5);
    assert_eq!(state.due_at, t0() + Duration::hours(12));
}

#[test]
fn ease_never_drops_below_minimum() {
    let state = run(&[Grade::Again; 20]);
    assert_eq!(state.ease_factor, MIN_EASE);
}

#[test]
fn due_boundary_is_inclusive() {
    let state = schedule(&ScheduleState::new(t0()), Grade::Good, t0());
    assert!(!state.is_due(state.due_at - Duration::seconds(1)));
    assert!(state.is_due(state.due_at));
    assert!(state.is_due(state.due_at + Duration::seconds(1)));
}

#[test]
fn reaching_three_weeks_marks_mastered() {
    let state = run(&[Grade::Easy, Grade::Easy, Grade::Easy, Grade::Easy]);
    assert!(state.interval_days >= 21.0, "{}", state.interval_days);
    assert_eq!(state.status, Status::Mastered);
}
