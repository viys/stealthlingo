//! Question selection and the interactive study loop (plain line input).

use std::collections::{HashSet, VecDeque};
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::{DateTime, Utc};

use super::scheduling::{schedule, Grade};
use super::spelling::{is_correct_spelling, mask_word};
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
/// are saved); a second one exits immediately.
pub fn install_interrupt_handler() {
    let _ = ctrlc::set_handler(|| {
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
    pub fn practice_mode(self) -> PracticeMode {
        match self {
            Self::Memory => PracticeMode::Memory,
            Self::Spelling => PracticeMode::Spelling,
            Self::Listening => PracticeMode::ListeningSpelling,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Memory => "Memory",
            Self::Spelling => "Spelling",
            Self::Listening => "Listening spelling",
        }
    }
}

/// Picks the words for a session: due reviews first, then a limited number of
/// never-studied words. Listening only uses words with audio; spelling needs a
/// definition to show as the prompt.
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
    if mode == StudyMode::Spelling {
        queue.retain(|item| item.entry.primary_definition().is_some());
    }
    Ok(queue)
}

#[derive(Debug, Clone)]
pub struct SessionPlan {
    pub mode: StudyMode,
    pub time_limit: Option<Duration>,
    pub max_answers: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct SessionSummary {
    pub answered: usize,
    pub correct: usize,
    pub skipped: usize,
    pub stopped_early: bool,
    pub time_up: bool,
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
    Answered {
        submitted: Option<String>,
        is_correct: bool,
        grade: Grade,
    },
    Skipped,
    Quit,
}

struct Question<'a> {
    item: &'a StudyItem,
    audio: &'a AudioPlayer,
}

impl Question<'_> {
    fn headword(&self) -> String {
        normalize_headword(&self.item.entry.word)
    }

    fn play_audio(&self) -> bool {
        let Some(url) = self.item.entry.audio_url() else {
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
    let started = Instant::now();
    let mut queue: VecDeque<StudyItem> = queue.into();
    let mut retried: HashSet<i64> = HashSet::new();
    let mut summary = SessionSummary::default();
    let mut shown = 0usize;

    while let Some(mut item) = queue.pop_front() {
        if interrupted() {
            summary.stopped_early = true;
            break;
        }
        if plan.max_answers.is_some_and(|max| summary.answered >= max) {
            break;
        }
        if plan
            .time_limit
            .is_some_and(|limit| started.elapsed() >= limit)
        {
            summary.time_up = true;
            break;
        }

        shown += 1;
        let remaining = match plan.max_answers {
            Some(max) => (max - summary.answered).min(queue.len() + 1),
            None => queue.len() + 1,
        };
        print_header(plan, shown, remaining, started);

        let question = Question { item: &item, audio };
        let asked_at = Instant::now();
        let outcome = match plan.mode {
            StudyMode::Memory => ask_memory(&question)?,
            StudyMode::Spelling => ask_spelling(&question)?,
            StudyMode::Listening => ask_listening(&question)?,
        };

        let (submitted, is_correct, grade) = match outcome {
            Outcome::Quit => {
                summary.stopped_early = true;
                break;
            }
            Outcome::Skipped => {
                summary.skipped += 1;
                continue;
            }
            Outcome::Answered {
                submitted,
                is_correct,
                grade,
            } => (submitted, is_correct, grade),
        };

        let now = Utc::now();
        let next = schedule(&item.schedule, grade, now);
        let expected = item.entry.word.clone();
        db.record_attempt(
            &NewAttempt {
                word_id: item.word_id,
                mode: plan.mode.practice_mode(),
                expected: &expected,
                submitted: submitted.as_deref(),
                is_correct,
                grade,
                duration_ms: Some(asked_at.elapsed().as_millis().min(i64::MAX as u128) as i64),
            },
            &next,
            now,
        )?;
        summary.answered += 1;
        if is_correct {
            summary.correct += 1;
        }
        println!("Next review: {}", describe_due(next.due_at, now));
        item.schedule = next;

        // A missed word comes back once at the end of this session.
        if !is_correct && retried.insert(item.word_id) {
            queue.push_back(item);
        }
    }

    if queue.is_empty() && !summary.stopped_early && !summary.time_up {
        println!();
        println!("All done for now.");
    } else if summary.time_up {
        println!();
        println!("Time's up.");
    }
    Ok(summary)
}

fn print_header(plan: &SessionPlan, shown: usize, remaining: usize, started: Instant) {
    println!();
    let mut header = format!("{} · #{shown} · {remaining} left", plan.mode.label());
    if let Some(limit) = plan.time_limit {
        let left = limit.saturating_sub(started.elapsed()).as_secs();
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
                    return Ok(Outcome::Answered {
                        submitted: None,
                        is_correct: grade != Grade::Again,
                        grade,
                    });
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
                    "?" => {
                        return Ok(Err(Outcome::Answered {
                            submitted: None,
                            is_correct: false,
                            grade: Grade::Again,
                        }))
                    }
                    _ => return Ok(Ok(trimmed.to_string())),
                }
            }
        }
    }
}

fn judge(q: &Question<'_>, attempt: std::result::Result<String, Outcome>) -> Outcome {
    let entry = &q.item.entry;
    let outcome = match attempt {
        Ok(answer) => {
            let correct = is_correct_spelling(&entry.word, &answer);
            Outcome::Answered {
                submitted: Some(answer),
                is_correct: correct,
                grade: if correct { Grade::Good } else { Grade::Again },
            }
        }
        Err(outcome) => outcome,
    };
    if let Outcome::Answered { is_correct, .. } = &outcome {
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

fn ask_spelling(q: &Question<'_>) -> Result<Outcome> {
    let entry = &q.item.entry;
    print_details(entry, q.item.note.as_deref(), true);
    let letters = entry.word.chars().filter(|c| c.is_alphabetic()).count();
    let words = entry.word.split_whitespace().count();
    if words > 1 {
        println!("Hint: {words} words, {letters} letters");
    } else {
        println!("Hint: {letters} letters");
    }
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
    if let Outcome::Answered {
        is_correct: true, ..
    } = outcome
    {
        println!("{}", q.item.entry.short_summary());
    }
    Ok(outcome)
}
