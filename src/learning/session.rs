//! Question selection and the interactive study loop (plain line input).

use std::collections::{HashSet, VecDeque};
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::{DateTime, Utc};

use super::scheduling::{schedule, Grade, ScheduleState};
use super::spelling::{is_correct_spelling, mask_word, missing_letters};
use super::PracticeMode;
use crate::audio::AudioPlayer;
use crate::cli::StudyMode;
use crate::dictionary::{normalize_headword, Entry};
use crate::storage::{Database, NewAttempt, StudyItem};
use crate::time::describe_due;

static SESSION_ACTIVE: AtomicBool = AtomicBool::new(false);
static INTERRUPTED: AtomicBool = AtomicBool::new(false);

/// Ctrl+C outside a session exits immediately. Inside a session the first
/// Ctrl+C ends the session after the current prompt (answers already submitted
/// are saved); a second one exits immediately. The full-screen interface
/// handles the signal itself so it can restore the terminal first.
pub fn install_interrupt_handler() {
    let _ = ctrlc::set_handler(|| {
        if crate::tui::on_interrupt_signal() {
            return;
        }
        if SESSION_ACTIVE.load(Ordering::SeqCst) && !INTERRUPTED.swap(true, Ordering::SeqCst) {
            eprintln!("\nStopping the session. Submitted answers are saved.");
        } else {
            std::process::exit(130);
        }
    });
}

fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

impl StudyMode {
    /// How answers in this mode are recorded. Missing-letter questions are
    /// spelling questions with a hint.
    pub fn practice_mode(self) -> PracticeMode {
        match self {
            Self::Memory | Self::Mixed => PracticeMode::Memory,
            Self::Spelling | Self::Letters => PracticeMode::Spelling,
            Self::Listening => PracticeMode::ListeningSpelling,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Memory => "Memory",
            Self::Spelling => "Spelling",
            Self::Letters => "Missing letters",
            Self::Listening => "Listening spelling",
            Self::Mixed => "Mixed",
        }
    }

    /// Whether the answer is typed rather than self-rated.
    pub fn is_typed(self) -> bool {
        matches!(self, Self::Spelling | Self::Letters | Self::Listening)
    }

    /// Words this mode can ask: listening needs audio, the spelling modes a
    /// definition to show as the prompt.
    fn can_ask(self, item: &StudyItem) -> bool {
        match self {
            Self::Memory | Self::Mixed => true,
            Self::Spelling | Self::Letters => item.entry.primary_definition().is_some(),
            Self::Listening => item.entry.audio_url().is_some(),
        }
    }
}

/// Picks the words for a session: due reviews first, then a limited number of
/// never-studied words, keeping only words the mode can ask.
pub fn build_queue(
    db: &Database,
    mode: StudyMode,
    due_only: bool,
    daily_new_limit: u32,
    now: DateTime<Utc>,
    day_start: DateTime<Utc>,
) -> Result<Vec<StudyItem>> {
    let audio_only = mode == StudyMode::Listening;
    let mut queue = db.due_items(now, audio_only)?;
    if !due_only {
        let started_today = db.count_new_started_since(day_start)?;
        let allowance = (i64::from(daily_new_limit) - started_today).max(0) as usize;
        if allowance > 0 {
            queue.extend(db.new_items(allowance, audio_only)?);
        }
    }
    queue.retain(|item| mode.can_ask(item));
    Ok(queue)
}

/// How far back `--mistakes` looks for wrong answers.
pub const MISTAKE_DAYS: i64 = 30;

/// Saved words answered wrong in the last [`MISTAKE_DAYS`] days or forgotten
/// after being learned (`lapses`), most recent mistake first, whether or not
/// they are due.
pub fn build_mistake_queue(
    db: &Database,
    mode: StudyMode,
    now: DateTime<Utc>,
) -> Result<Vec<StudyItem>> {
    let since = now - chrono::Duration::days(MISTAKE_DAYS);
    let mut queue = db.mistake_items(since, mode == StudyMode::Listening)?;
    queue.retain(|item| mode.can_ask(item));
    Ok(queue)
}

#[derive(Debug, Clone, Default)]
pub struct SessionPlan {
    pub mode: StudyMode,
    pub time_limit: Option<Duration>,
    pub max_answers: Option<usize>,
    /// Accent of the recordings played by default ("UK" or "US").
    pub accent: String,
    /// Different words to practise today; 0 means no goal.
    pub daily_goal: usize,
    /// Words already answered today before this session started.
    pub practised_today: HashSet<i64>,
    /// End the session once the daily goal is reached instead of on a timer.
    /// Answers count, not questions: a missed word asked again at the end
    /// adds nothing.
    pub until_goal: bool,
    /// The queue holds every saved word that can be practised today; false
    /// for reviews, mistakes and modes that skip some words, which can run
    /// out while other words are still available.
    pub whole_list: bool,
}

#[derive(Debug, Clone, Default)]
pub struct SessionSummary {
    pub answered: usize,
    pub correct: usize,
    pub skipped: usize,
    pub stopped_early: bool,
    pub time_up: bool,
    /// The session ended because today's goal was reached.
    pub goal_done: bool,
    /// Today's goal was reached by an answer in this session.
    pub goal_reached_now: bool,
    /// Words answered wrong at least once, in the order they were missed.
    pub missed: Vec<String>,
}

/// A submitted answer to one question.
#[derive(Debug, Clone)]
pub struct Answer {
    pub submitted: Option<String>,
    pub is_correct: bool,
    pub grade: Grade,
}

/// Question order, limits and results of one session, independent of how
/// questions are shown. Each answer is stored as soon as it is recorded.
pub struct Session {
    plan: SessionPlan,
    queue: VecDeque<StudyItem>,
    retried: HashSet<i64>,
    /// Words answered today, including before this session.
    practised: HashSet<i64>,
    summary: SessionSummary,
    started: Instant,
    paused_for: Duration,
    paused_at: Option<Instant>,
    shown: usize,
    /// How the current question is asked; differs per word in mixed practice.
    current_mode: StudyMode,
}

impl Session {
    pub fn new(plan: SessionPlan, queue: Vec<StudyItem>) -> Self {
        Self {
            current_mode: plan.mode,
            practised: plan.practised_today.clone(),
            plan,
            queue: queue.into(),
            retried: HashSet::new(),
            summary: SessionSummary::default(),
            started: Instant::now(),
            paused_for: Duration::ZERO,
            paused_at: None,
            shown: 0,
        }
    }

    pub fn plan(&self) -> &SessionPlan {
        &self.plan
    }

    pub fn summary(&self) -> &SessionSummary {
        &self.summary
    }

    /// Questions shown so far, including the current one.
    pub fn shown(&self) -> usize {
        self.shown
    }

    /// Questions still to come, including the current one.
    pub fn remaining(&self) -> usize {
        let queued = self.queue.len() + 1;
        let queued = match self.plan.max_answers {
            Some(max) => max.saturating_sub(self.summary.answered).min(queued),
            None => queued,
        };
        if self.plan.until_goal {
            queued.min(self.words_to_go().max(1))
        } else {
            queued
        }
    }

    /// Different words answered today, this session included.
    pub fn words_today(&self) -> usize {
        self.practised.len()
    }

    /// Words still missing from today's goal.
    pub fn words_to_go(&self) -> usize {
        self.plan.daily_goal.saturating_sub(self.words_today())
    }

    fn goal_reached(&self) -> bool {
        self.plan.daily_goal > 0 && self.words_to_go() == 0
    }

    /// A line about today's goal for the end of the session, if one is set.
    pub fn goal_message(&self) -> Option<String> {
        let goal = self.plan.daily_goal;
        if goal == 0 {
            return None;
        }
        let done = self.words_today();
        if self.summary.goal_reached_now {
            return Some(format!(
                "Daily goal reached: {done} {} today (goal {goal}). Keep going any time.",
                if done == 1 { "word" } else { "words" }
            ));
        }
        if done >= goal {
            return Some(format!("Today: {done} / {goal} words · goal reached"));
        }
        let short = goal - done;
        let words = if short == 1 { "word" } else { "words" };
        if self.plan.until_goal
            && self.plan.whole_list
            && self.queue.is_empty()
            && !self.summary.stopped_early
            && !self.summary.time_up
        {
            return Some(format!(
                "Nothing left to practise today, {short} {words} short of the goal. \
                 Raise daily_new_limit or save more words."
            ));
        }
        Some(format!("Today: {done} / {goal} words · {short} to go."))
    }

    pub fn queue_is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Study time so far, not counting pauses.
    pub fn elapsed(&self) -> Duration {
        let paused = self.paused_for + self.paused_at.map_or(Duration::ZERO, |at| at.elapsed());
        self.started.elapsed().saturating_sub(paused)
    }

    pub fn time_left(&self) -> Option<Duration> {
        self.plan
            .time_limit
            .map(|limit| limit.saturating_sub(self.elapsed()))
    }

    /// Stops the clock, e.g. while the screen is hidden.
    pub fn pause(&mut self) {
        self.paused_at.get_or_insert_with(Instant::now);
    }

    pub fn resume(&mut self) {
        if let Some(at) = self.paused_at.take() {
            self.paused_for += at.elapsed();
        }
    }

    /// The next question, or `None` when the queue is empty or a limit is reached.
    pub fn next_item(&mut self) -> Option<StudyItem> {
        if self.summary.stopped_early {
            return None;
        }
        if self
            .plan
            .max_answers
            .is_some_and(|max| self.summary.answered >= max)
        {
            return None;
        }
        if self.plan.until_goal && self.goal_reached() {
            self.summary.goal_done = true;
            return None;
        }
        if self.time_left() == Some(Duration::ZERO) {
            self.summary.time_up = true;
            return None;
        }
        let item = self.queue.pop_front()?;
        self.shown += 1;
        self.current_mode = self.pick_mode(&item);
        Some(item)
    }

    /// How the current question is asked.
    pub fn current_mode(&self) -> StudyMode {
        self.current_mode
    }

    /// Mixed practice shows new words as flashcards first; later it rotates
    /// through the modes the word supports.
    fn pick_mode(&self, item: &StudyItem) -> StudyMode {
        if self.plan.mode != StudyMode::Mixed {
            return self.plan.mode;
        }
        if item.schedule.status == crate::learning::Status::New {
            return StudyMode::Memory;
        }
        let options: Vec<StudyMode> = [
            StudyMode::Memory,
            StudyMode::Spelling,
            StudyMode::Letters,
            StudyMode::Listening,
        ]
        .into_iter()
        .filter(|mode| mode.can_ask(item))
        .collect();
        options[(item.word_id.unsigned_abs() as usize + self.shown) % options.len()]
    }

    /// Stores an answer and returns the new schedule. A missed word comes back
    /// once at the end of this session.
    pub fn record(
        &mut self,
        db: &mut Database,
        mut item: StudyItem,
        answer: &Answer,
        took: Duration,
    ) -> Result<ScheduleState> {
        let now = Utc::now();
        let next = schedule(&item.schedule, answer.grade, now);
        db.record_attempt(
            &NewAttempt {
                word_id: item.word_id,
                mode: self.current_mode.practice_mode(),
                expected: &item.entry.word,
                submitted: answer.submitted.as_deref(),
                is_correct: answer.is_correct,
                grade: answer.grade,
                duration_ms: Some(took.as_millis().min(i64::MAX as u128) as i64),
            },
            &next,
            now,
        )?;
        self.summary.answered += 1;
        let before = self.goal_reached();
        self.practised.insert(item.word_id);
        if !before && self.goal_reached() {
            self.summary.goal_reached_now = true;
        }
        if answer.is_correct {
            self.summary.correct += 1;
        } else if !self.summary.missed.contains(&item.entry.word) {
            self.summary.missed.push(item.entry.word.clone());
        }
        item.schedule = next.clone();
        if !answer.is_correct && self.retried.insert(item.word_id) {
            self.queue.push_back(item);
        }
        Ok(next)
    }

    pub fn skip(&mut self) {
        self.summary.skipped += 1;
    }

    /// Ends the session before the queue is done.
    pub fn stop(&mut self) {
        self.summary.stopped_early = true;
    }
}

/// "9 letters", "2 words, 8 letters", or with `blanks` the word with some
/// letters missing.
pub fn spelling_hint(word: &str, blanks: bool) -> String {
    if blanks {
        return missing_letters(word);
    }
    let letters = word.chars().filter(|c| c.is_alphabetic()).count();
    match word.split_whitespace().count() {
        n if n > 1 => format!("{n} words, {letters} letters"),
        _ => format!("{letters} letters"),
    }
}

/// Text answers are graded Good when correct and Again when wrong.
pub fn judge_spelling(expected: &str, submitted: &str) -> Answer {
    let is_correct = is_correct_spelling(expected, submitted);
    Answer {
        submitted: Some(submitted.trim().to_string()),
        is_correct,
        grade: if is_correct {
            Grade::Good
        } else {
            Grade::Again
        },
    }
}

/// A self-rated flashcard answer; only Again counts as a miss.
pub fn memory_answer(grade: Grade) -> Answer {
    Answer {
        submitted: None,
        is_correct: grade != Grade::Again,
        grade,
    }
}

/// Giving up on a typed answer counts as a miss.
pub fn gave_up() -> Answer {
    Answer {
        submitted: None,
        is_correct: false,
        grade: Grade::Again,
    }
}

enum Input {
    Text(String),
    Quit,
}

fn read_input(prompt: &str) -> Result<Input> {
    if interrupted() {
        return Ok(Input::Quit);
    }
    print!("{prompt}");
    io::stdout().flush()?;
    let mut line = String::new();
    let read = io::stdin().lock().read_line(&mut line);
    if interrupted() {
        return Ok(Input::Quit);
    }
    match read {
        Ok(0) => Ok(Input::Quit),
        Ok(_) => Ok(Input::Text(
            line.trim_start_matches('\u{feff}')
                .trim_end_matches(['\r', '\n'])
                .to_string(),
        )),
        Err(err) if err.kind() == io::ErrorKind::Interrupted => Ok(Input::Quit),
        Err(err) => Err(err.into()),
    }
}

enum Outcome {
    Answered(Answer),
    Skipped,
    Quit,
}

struct Question<'a> {
    item: &'a StudyItem,
    audio: &'a AudioPlayer,
    accent: &'a str,
}

impl Question<'_> {
    fn headword(&self) -> String {
        normalize_headword(&self.item.entry.word)
    }

    fn play_audio(&self) -> bool {
        let recording = self.item.entry.audio_in(Some(self.accent));
        let Some(url) = recording.and_then(|p| p.audio_url.as_deref()) else {
            println!("No pronunciation audio for this word.");
            return false;
        };
        match self.audio.play(&self.headword(), url) {
            Ok(()) => true,
            Err(err) => {
                println!("Could not play audio: {err:#}");
                false
            }
        }
    }
}

/// Runs the interactive loop. Each answer is stored as soon as it is submitted.
pub fn run_session(
    db: &mut Database,
    audio: &AudioPlayer,
    plan: &SessionPlan,
    queue: Vec<StudyItem>,
) -> Result<SessionSummary> {
    INTERRUPTED.store(false, Ordering::SeqCst);
    SESSION_ACTIVE.store(true, Ordering::SeqCst);
    let result = session_loop(db, audio, plan, queue);
    SESSION_ACTIVE.store(false, Ordering::SeqCst);
    result
}

fn session_loop(
    db: &mut Database,
    audio: &AudioPlayer,
    plan: &SessionPlan,
    queue: Vec<StudyItem>,
) -> Result<SessionSummary> {
    let mut session = Session::new(plan.clone(), queue);

    loop {
        if interrupted() {
            session.stop();
            break;
        }
        let Some(item) = session.next_item() else {
            break;
        };
        print_header(&session);

        let question = Question {
            item: &item,
            audio,
            accent: &plan.accent,
        };
        let asked_at = Instant::now();
        let outcome = match session.current_mode() {
            StudyMode::Memory | StudyMode::Mixed => ask_memory(&question)?,
            StudyMode::Spelling => ask_spelling(&question, false)?,
            StudyMode::Letters => ask_spelling(&question, true)?,
            StudyMode::Listening => ask_listening(&question)?,
        };

        let answer = match outcome {
            Outcome::Quit => {
                session.stop();
                break;
            }
            Outcome::Skipped => {
                session.skip();
                continue;
            }
            Outcome::Answered(answer) => answer,
        };
        let next = session.record(db, item, &answer, asked_at.elapsed())?;
        println!("Next review: {}", describe_due(next.due_at, Utc::now()));
    }

    let summary = session.summary().clone();
    if summary.time_up {
        println!();
        println!("Time's up.");
    } else if summary.goal_done {
        println!();
        println!("That's today's goal.");
    } else if session.queue_is_empty() && !summary.stopped_early {
        println!();
        println!("All done for now.");
    }
    if let Some(line) = session.goal_message() {
        println!("{line}");
    }
    Ok(summary)
}

fn print_header(session: &Session) {
    println!();
    let mut header = format!(
        "{} · #{} · {} left",
        session.current_mode().label(),
        session.shown(),
        session.remaining()
    );
    if let Some(left) = session.time_left() {
        let left = left.as_secs();
        header.push_str(&format!(" · {}:{:02} remaining", left / 60, left % 60));
    }
    println!("{header}");
    println!("{}", "-".repeat(header.chars().count()));
}

fn grade_from_input(input: &str) -> Option<Grade> {
    match input {
        "1" => Some(Grade::Again),
        "2" => Some(Grade::Hard),
        "3" => Some(Grade::Good),
        "4" => Some(Grade::Easy),
        _ => None,
    }
}

fn print_details(entry: &Entry, note: Option<&str>, mask: bool) {
    let hide = |text: &str| {
        if mask {
            mask_word(text, &entry.word)
        } else {
            text.to_string()
        }
    };
    let mut shown_parts = 0;
    for meaning in &entry.meanings {
        let Some(definition) = meaning.definitions.first() else {
            continue;
        };
        let pos = meaning.part_of_speech.as_deref().unwrap_or("meaning");
        println!("{pos}: {}", hide(&definition.definition));
        if let Some(example) = &definition.example {
            println!("  Example: {}", hide(example));
        }
        shown_parts += 1;
        if shown_parts == 2 {
            break;
        }
    }
    if shown_parts == 0 {
        println!("(no definition available)");
    }
    if let Some(note) = note {
        println!("Note: {note}");
    }
}

fn print_answer(entry: &Entry) {
    match &entry.phonetic {
        Some(phonetic) => println!("{}  {phonetic}", entry.word),
        None => println!("{}", entry.word),
    }
}

fn ask_memory(q: &Question<'_>) -> Result<Outcome> {
    let entry = &q.item.entry;
    print_answer(entry);
    let audio_hint = if entry.has_audio() { "  [a] audio" } else { "" };
    loop {
        match read_input(&format!(
            "[Enter] reveal{audio_hint}  [s] skip  [q] quit > "
        ))? {
            Input::Quit => return Ok(Outcome::Quit),
            Input::Text(text) => match text.trim().to_lowercase().as_str() {
                "" => break,
                "q" => return Ok(Outcome::Quit),
                "s" => return Ok(Outcome::Skipped),
                "a" => {
                    q.play_audio();
                }
                _ => {}
            },
        }
    }
    print_details(entry, q.item.note.as_deref(), false);
    loop {
        match read_input("[1] Again  [2] Hard  [3] Good  [4] Easy  [q] quit > ")? {
            Input::Quit => return Ok(Outcome::Quit),
            Input::Text(text) => {
                let text = text.trim().to_lowercase();
                if text == "q" {
                    return Ok(Outcome::Quit);
                }
                if text == "a" && entry.has_audio() {
                    q.play_audio();
                    continue;
                }
                if let Some(grade) = grade_from_input(&text) {
                    return Ok(Outcome::Answered(memory_answer(grade)));
                }
                println!("Please choose 1-4.");
            }
        }
    }
}

/// Reads a typed answer. `allow_replay` enables the `r` command.
fn read_spelling(
    q: &Question<'_>,
    allow_replay: bool,
) -> Result<std::result::Result<String, Outcome>> {
    let replay = if allow_replay { "  [r] replay" } else { "" };
    loop {
        let prompt = format!("Your spelling ([?] give up{replay}  [s] skip  [q] quit) > ");
        match read_input(&prompt)? {
            Input::Quit => return Ok(Err(Outcome::Quit)),
            Input::Text(text) => {
                let trimmed = text.trim();
                match trimmed.to_lowercase().as_str() {
                    "" => println!("Type the word, or ? to give up."),
                    "q" => return Ok(Err(Outcome::Quit)),
                    "s" => return Ok(Err(Outcome::Skipped)),
                    "r" if allow_replay => {
                        q.play_audio();
                    }
                    "?" => return Ok(Err(Outcome::Answered(gave_up()))),
                    _ => return Ok(Ok(trimmed.to_string())),
                }
            }
        }
    }
}

fn judge(q: &Question<'_>, attempt: std::result::Result<String, Outcome>) -> Outcome {
    let entry = &q.item.entry;
    let outcome = match attempt {
        Ok(answer) => Outcome::Answered(judge_spelling(&entry.word, &answer)),
        Err(outcome) => outcome,
    };
    if let Outcome::Answered(Answer { is_correct, .. }) = &outcome {
        if *is_correct {
            println!("Correct!");
        } else {
            println!("Not quite. The answer is:");
        }
        print_answer(entry);
        if !*is_correct {
            println!("{}", entry.short_summary());
        }
    }
    outcome
}

/// Shows the definition with the word masked; `blanks` adds the word with
/// some letters missing.
fn ask_spelling(q: &Question<'_>, blanks: bool) -> Result<Outcome> {
    let entry = &q.item.entry;
    print_details(entry, q.item.note.as_deref(), true);
    println!("Hint: {}", spelling_hint(&entry.word, blanks));
    let attempt = read_spelling(q, false)?;
    Ok(judge(q, attempt))
}

fn ask_listening(q: &Question<'_>) -> Result<Outcome> {
    if q.item.entry.audio_url().is_none() {
        return Ok(Outcome::Skipped);
    }
    println!("Playing pronunciation...");
    if !q.play_audio() {
        println!("Press r to retry the audio or s to skip this word.");
    }
    let attempt = read_spelling(q, true)?;
    let outcome = judge(q, attempt);
    if let Outcome::Answered(Answer {
        is_correct: true, ..
    }) = outcome
    {
        println!("{}", q.item.entry.short_summary());
    }
    Ok(outcome)
}
