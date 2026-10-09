use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};

use super::Database;
use crate::dictionary::{decode_cached, Entry};
use crate::learning::{Grade, PracticeMode, ScheduleState, Status};
use crate::time::{from_db, to_db};

pub const LANGUAGE: &str = "en";

/// A dictionary entry stored in the local cache.
#[derive(Debug, Clone)]
pub struct CachedWord {
    pub id: i64,
    pub headword: String,
    pub entry: Entry,
    /// Dictionary the entry came from; anything other than
    /// [`crate::dictionary::SOURCE`] is refreshed when used (older dictionaries and
    /// [`crate::dictionary::PARTIAL_SOURCE`]).
    pub source: String,
    pub fetched_at: DateTime<Utc>,
    pub in_collection: bool,
    pub note: Option<String>,
}

/// A saved word with everything a study session needs.
#[derive(Debug, Clone)]
pub struct StudyItem {
    pub word_id: i64,
    pub entry: Entry,
    pub note: Option<String>,
    pub schedule: ScheduleState,
}

/// A row of the `words` listing.
#[derive(Debug, Clone)]
pub struct WordSummary {
    pub word_id: i64,
    pub display_word: String,
    pub status: Status,
    pub due_at: DateTime<Utc>,
    pub note: Option<String>,
    pub summary: String,
    pub has_audio: bool,
    pub repetitions: u32,
    pub interval_days: f64,
}

#[derive(Debug, Clone)]
pub struct NewAttempt<'a> {
    pub word_id: i64,
    pub mode: PracticeMode,
    pub expected: &'a str,
    pub submitted: Option<&'a str>,
    pub is_correct: bool,
    pub grade: Grade,
    pub duration_ms: Option<i64>,
}

/// Columns of `user_words`, shared with `archived_user_words`.
const SCHEDULE_COLUMNS: &str = "word_id, status, personal_note, due_at, interval_days, \
     repetitions, ease_factor, lapses, added_at, last_reviewed_at";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddOutcome {
    Added,
    /// Re-added after a link removal, with the archived study progress.
    Restored,
    AlreadySaved,
    NoteUpdated,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stats {
    pub total_words: i64,
    pub status_counts: Vec<(Status, i64)>,
    pub due_now: i64,
    pub attempts_today: i64,
    pub correct_today: i64,
    pub new_words_today: i64,
    pub total_attempts: i64,
    pub total_correct: i64,
    pub next_due: Option<DateTime<Utc>>,
}

/// Escapes `%`, `_` and `\` for use inside a LIKE pattern with `ESCAPE '\'`.
fn like_pattern(query: &str) -> String {
    let mut escaped = String::with_capacity(query.len() + 2);
    escaped.push('%');
    for ch in query.chars() {
        if matches!(ch, '%' | '_' | '\\') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped.push('%');
    escaped
}

fn parse_entry(display_word: &str, raw: &str, entry_json: Option<&str>) -> Result<Entry> {
    decode_cached(display_word, raw, entry_json)
        .map_err(|e| anyhow!("cached dictionary data for \"{display_word}\" is corrupt: {e}"))
}

fn parse_status(value: &str) -> Result<Status> {
    Status::parse(value).with_context(|| format!("unknown word status \"{value}\" in database"))
}

struct StudyRow {
    word_id: i64,
    display_word: String,
    raw_json: String,
    note: Option<String>,
    status: String,
    repetitions: u32,
    interval_days: f64,
    ease_factor: f64,
    lapses: u32,
    due_at: String,
    last_reviewed_at: Option<String>,
    entry_json: Option<String>,
}

const STUDY_SELECT: &str =
    "SELECT w.id, w.display_word, w.raw_response_json, u.personal_note, u.status,
        u.repetitions, u.interval_days, u.ease_factor, u.lapses, u.due_at, u.last_reviewed_at,
        w.entry_json
    FROM user_words u JOIN words w ON w.id = u.word_id";

impl StudyRow {
    fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            word_id: row.get(0)?,
            display_word: row.get(1)?,
            raw_json: row.get(2)?,
            note: row.get(3)?,
            status: row.get(4)?,
            repetitions: row.get(5)?,
            interval_days: row.get(6)?,
            ease_factor: row.get(7)?,
            lapses: row.get(8)?,
            due_at: row.get(9)?,
            last_reviewed_at: row.get(10)?,
            entry_json: row.get(11)?,
        })
    }

    fn into_item(self) -> Result<StudyItem> {
        Ok(StudyItem {
            word_id: self.word_id,
            entry: parse_entry(
                &self.display_word,
                &self.raw_json,
                self.entry_json.as_deref(),
            )?,
            note: self.note,
            schedule: ScheduleState {
                status: parse_status(&self.status)?,
                repetitions: self.repetitions,
                interval_days: self.interval_days,
                ease_factor: self.ease_factor,
                lapses: self.lapses,
                due_at: from_db(&self.due_at)?,
                last_reviewed_at: self.last_reviewed_at.as_deref().map(from_db).transpose()?,
            },
        })
    }
}

impl Database {
    /// Inserts or refreshes a Wiktionary entry in the cache and returns its id.
    /// `source` is [`crate::dictionary::SOURCE`], or [`crate::dictionary::PARTIAL_SOURCE`]
    /// when the pronunciation data is missing.
    pub fn cache_word(
        &self,
        headword: &str,
        entry: &Entry,
        raw_json: &str,
        source: &str,
        now: DateTime<Utc>,
    ) -> Result<i64> {
        let entry_json = serde_json::to_string(entry)?;
        let id = self.conn.query_row(
            "INSERT INTO words (language_code, headword_normalized, display_word, raw_response_json,
                                has_audio, source_fetched_at, created_at, source, entry_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?7, ?8)
             ON CONFLICT (language_code, headword_normalized) DO UPDATE SET
                display_word = excluded.display_word,
                raw_response_json = excluded.raw_response_json,
                has_audio = excluded.has_audio,
                source_fetched_at = excluded.source_fetched_at,
                source = excluded.source,
                entry_json = excluded.entry_json
             RETURNING id",
            params![
                LANGUAGE,
                headword,
                entry.word,
                raw_json,
                entry.has_audio(),
                to_db(now),
                source,
                entry_json
            ],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    /// Changes the recorded dictionary source of a cached word.
    pub fn set_source(&self, word_id: i64, source: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE words SET source = ?2 WHERE id = ?1",
            params![word_id, source],
        )?;
        Ok(())
    }

    /// Remembers that looking up `alias` produced the cached word `word_id`.
    /// Does nothing when `alias` is already the word's own headword.
    pub fn add_alias(&self, alias: &str, word_id: i64) -> Result<()> {
        self.conn.execute(
            "INSERT INTO word_aliases (language_code, alias, word_id)
             SELECT ?1, ?2, ?3
             WHERE NOT EXISTS (SELECT 1 FROM words WHERE id = ?3 AND headword_normalized = ?2)
             ON CONFLICT (language_code, alias) DO UPDATE SET word_id = excluded.word_id",
            params![LANGUAGE, alias, word_id],
        )?;
        Ok(())
    }

    /// Id of the cached word for `headword`: an exact headword match wins over
    /// an alias.
    fn resolve_word_id(&self, headword: &str) -> Result<Option<i64>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, 0 AS rank FROM words
                    WHERE language_code = ?1 AND headword_normalized = ?2
                 UNION ALL
                 SELECT word_id, 1 FROM word_aliases WHERE language_code = ?1 AND alias = ?2
                 ORDER BY rank LIMIT 1",
                params![LANGUAGE, headword],
                |row| row.get(0),
            )
            .optional()?)
    }

    pub fn find_cached(&self, headword: &str) -> Result<Option<CachedWord>> {
        let Some(word_id) = self.resolve_word_id(headword)? else {
            return Ok(None);
        };
        let row = self
            .conn
            .query_row(
                "SELECT w.id, w.headword_normalized, w.display_word, w.raw_response_json,
                        w.source_fetched_at, u.word_id IS NOT NULL, u.personal_note,
                        w.source, w.entry_json
                 FROM words w LEFT JOIN user_words u ON u.word_id = w.id
                 WHERE w.id = ?1",
                params![word_id],
                |row| {
                    Ok((
                        (
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                        ),
                        row.get::<_, String>(4)?,
                        row.get::<_, bool>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, Option<String>>(8)?,
                    ))
                },
            )
            .optional()?;
        row.map(
            |((id, headword, display, raw), fetched, in_collection, note, source, entry_json)| {
                Ok(CachedWord {
                    id,
                    headword,
                    entry: parse_entry(&display, &raw, entry_json.as_deref())?,
                    source,
                    fetched_at: from_db(&fetched)?,
                    in_collection,
                    note,
                })
            },
        )
        .transpose()
    }

    /// Adds a cached word to the study list. Saving the same word twice is a no-op
    /// (apart from updating the note when one is supplied). A word removed with
    /// [`Database::archive_from_collection`] comes back with its study progress.
    pub fn add_to_collection(
        &self,
        word_id: i64,
        note: Option<&str>,
        now: DateTime<Utc>,
    ) -> Result<AddOutcome> {
        let note = note.map(str::trim).filter(|n| !n.is_empty());
        let tx = self.conn.unchecked_transaction()?;
        let restored = tx.execute(
            &format!(
                "INSERT INTO user_words ({SCHEDULE_COLUMNS})
                 SELECT {SCHEDULE_COLUMNS} FROM archived_user_words WHERE word_id = ?1
                 ON CONFLICT (word_id) DO NOTHING"
            ),
            [word_id],
        )?;
        tx.execute(
            "DELETE FROM archived_user_words WHERE word_id = ?1",
            [word_id],
        )?;
        let outcome = if restored == 1 {
            AddOutcome::Restored
        } else {
            let inserted = tx.execute(
                "INSERT INTO user_words (word_id, status, personal_note, due_at, added_at)
                 VALUES (?1, 'new', ?2, ?3, ?3)
                 ON CONFLICT (word_id) DO NOTHING",
                params![word_id, note, to_db(now)],
            )?;
            match (inserted, note) {
                (1, _) => AddOutcome::Added,
                (_, Some(_)) => AddOutcome::NoteUpdated,
                (_, None) => AddOutcome::AlreadySaved,
            }
        };
        if let (AddOutcome::Restored | AddOutcome::NoteUpdated, Some(note)) = (outcome, note) {
            tx.execute(
                "UPDATE user_words SET personal_note = ?2 WHERE word_id = ?1",
                params![word_id, note],
            )?;
        }
        tx.commit()?;
        Ok(outcome)
    }

    /// Removes a word from the study list but keeps its study progress, so
    /// adding it again restores it. Used for removals triggered by links,
    /// which other programs could also open. Returns false if the word was
    /// not saved.
    pub fn archive_from_collection(&self, headword: &str, now: DateTime<Utc>) -> Result<bool> {
        let Some(word_id) = self.resolve_word_id(headword)? else {
            return Ok(false);
        };
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            &format!(
                "INSERT OR REPLACE INTO archived_user_words ({SCHEDULE_COLUMNS}, archived_at)
                 SELECT {SCHEDULE_COLUMNS}, ?2 FROM user_words WHERE word_id = ?1"
            ),
            params![word_id, to_db(now)],
        )?;
        let removed = tx.execute("DELETE FROM user_words WHERE word_id = ?1", [word_id])?;
        tx.commit()?;
        Ok(removed > 0)
    }

    /// Removes a word from the study list. The dictionary cache and answer
    /// history are kept. Returns false if the word was not saved.
    pub fn remove_from_collection(&self, headword: &str) -> Result<bool> {
        let Some(word_id) = self.resolve_word_id(headword)? else {
            return Ok(false);
        };
        let removed = self
            .conn
            .execute("DELETE FROM user_words WHERE word_id = ?1", [word_id])?;
        Ok(removed > 0)
    }

    pub fn collection_size(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM user_words", [], |row| row.get(0))?)
    }

    /// Lists saved words, optionally filtered by a substring of the word or note.
    pub fn list_words(&self, query: Option<&str>) -> Result<Vec<WordSummary>> {
        let pattern = query
            .map(|q| q.trim().to_lowercase())
            .filter(|q| !q.is_empty())
            .map(|q| like_pattern(&q));
        let mut stmt = self.conn.prepare(
            "SELECT w.id, w.display_word, w.raw_response_json, w.has_audio, u.status, u.due_at,
                    u.personal_note, u.repetitions, u.interval_days, w.entry_json
             FROM user_words u JOIN words w ON w.id = u.word_id
             WHERE ?1 IS NULL
                OR w.headword_normalized LIKE ?1 ESCAPE '\\'
                OR lower(coalesce(u.personal_note, '')) LIKE ?1 ESCAPE '\\'
             ORDER BY w.headword_normalized",
        )?;
        let rows = stmt
            .query_map(params![pattern], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, bool>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, u32>(7)?,
                    row.get::<_, f64>(8)?,
                    row.get::<_, Option<String>>(9)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows.into_iter()
            .map(
                |(id, display, raw, has_audio, status, due, note, reps, interval, entry_json)| {
                    Ok(WordSummary {
                        word_id: id,
                        summary: parse_entry(&display, &raw, entry_json.as_deref())?
                            .short_summary(),
                        display_word: display,
                        status: parse_status(&status)?,
                        due_at: from_db(&due)?,
                        note,
                        has_audio,
                        repetitions: reps,
                        interval_days: interval,
                    })
                },
            )
            .collect()
    }

    /// Previously studied words whose review time has arrived, oldest first.
    pub fn due_items(&self, now: DateTime<Utc>, audio_only: bool) -> Result<Vec<StudyItem>> {
        let sql = format!(
            "{STUDY_SELECT} WHERE u.status != 'new' AND u.due_at <= ?1 AND (?2 = 0 OR w.has_audio = 1)
             ORDER BY u.due_at, w.id"
        );
        self.study_query(&sql, params![to_db(now), audio_only])
    }

    /// Saved words that have never been studied, in the order they were added.
    pub fn new_items(&self, limit: usize, audio_only: bool) -> Result<Vec<StudyItem>> {
        let sql = format!(
            "{STUDY_SELECT} WHERE u.status = 'new' AND (?2 = 0 OR w.has_audio = 1)
             ORDER BY u.added_at, w.id LIMIT ?1"
        );
        self.study_query(&sql, params![limit as i64, audio_only])
    }

    fn study_query(&self, sql: &str, params: impl rusqlite::Params) -> Result<Vec<StudyItem>> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt
            .query_map(params, StudyRow::from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows.into_iter().map(StudyRow::into_item).collect()
    }

    pub fn count_due(&self, now: DateTime<Utc>) -> Result<i64> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM user_words WHERE status != 'new' AND due_at <= ?1",
            params![to_db(now)],
            |row| row.get(0),
        )?)
    }

    pub fn count_new_ready(&self) -> Result<i64> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM user_words WHERE status = 'new'",
            [],
            |row| row.get(0),
        )?)
    }

    /// Number of words whose very first answer happened at or after `since`.
    pub fn count_new_started_since(&self, since: DateTime<Utc>) -> Result<i64> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM
                (SELECT MIN(created_at) AS first_at FROM practice_attempts GROUP BY word_id)
             WHERE first_at >= ?1",
            params![to_db(since)],
            |row| row.get(0),
        )?)
    }

    pub fn next_due_at(&self) -> Result<Option<DateTime<Utc>>> {
        let value: Option<String> = self.conn.query_row(
            "SELECT MIN(due_at) FROM user_words WHERE status != 'new'",
            [],
            |row| row.get(0),
        )?;
        value.as_deref().map(from_db).transpose()
    }

    /// Stores an answer and the resulting schedule atomically.
    pub fn record_attempt(
        &mut self,
        attempt: &NewAttempt<'_>,
        next: &ScheduleState,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO practice_attempts (word_id, mode, expected_answer, submitted_answer,
                                            is_correct, grade, duration_ms, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                attempt.word_id,
                attempt.mode.as_str(),
                attempt.expected,
                attempt.submitted,
                attempt.is_correct,
                attempt.grade.as_str(),
                attempt.duration_ms,
                to_db(now),
            ],
        )?;
        tx.execute(
            "UPDATE user_words SET status = ?2, due_at = ?3, interval_days = ?4, repetitions = ?5,
                    ease_factor = ?6, lapses = ?7, last_reviewed_at = ?8
             WHERE word_id = ?1",
            params![
                attempt.word_id,
                next.status.as_str(),
                to_db(next.due_at),
                next.interval_days,
                next.repetitions,
                next.ease_factor,
                next.lapses,
                next.last_reviewed_at.map(to_db),
            ],
        )?;
        tx.commit().context("could not save your answer")?;
        Ok(())
    }

    pub fn stats(&self, now: DateTime<Utc>, day_start: DateTime<Utc>) -> Result<Stats> {
        let mut stats = Stats {
            total_words: self.collection_size()?,
            due_now: self.count_due(now)?,
            new_words_today: self.count_new_started_since(day_start)?,
            next_due: self.next_due_at()?,
            ..Stats::default()
        };

        let mut stmt = self
            .conn
            .prepare("SELECT status, COUNT(*) FROM user_words GROUP BY status")?;
        let counts = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        stats.status_counts = Status::ALL
            .into_iter()
            .map(|s| {
                let n = counts
                    .iter()
                    .find(|(name, _)| name == s.as_str())
                    .map_or(0, |(_, n)| *n);
                (s, n)
            })
            .collect();

        let attempts = |since: Option<String>| -> Result<(i64, i64)> {
            Ok(self.conn.query_row(
                "SELECT COUNT(*), COALESCE(SUM(is_correct), 0) FROM practice_attempts
                 WHERE ?1 IS NULL OR created_at >= ?1",
                params![since],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?)
        };
        (stats.attempts_today, stats.correct_today) = attempts(Some(to_db(day_start)))?;
        (stats.total_attempts, stats.total_correct) = attempts(None)?;
        Ok(stats)
    }
}
