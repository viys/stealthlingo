//! Full JSON backup of the dictionary cache, study list and answer history.

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};

use super::Database;
use crate::dictionary::decode_cached;
use crate::learning::{Grade, PracticeMode, Status};
use crate::time::{from_db, to_db};

pub const BACKUP_FORMAT: &str = "stealthlingo-backup";
/// Version 2 added `source` and `entry_json`; version 1 files still import.
pub const BACKUP_VERSION: u32 = 2;

/// Version 1 backups only contain Free Dictionary responses.
fn default_source() -> String {
    "free-dictionary".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Backup {
    pub format: String,
    pub version: u32,
    pub exported_at: String,
    pub words: Vec<BackupWord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupWord {
    pub language_code: String,
    pub headword: String,
    pub display_word: String,
    pub raw_response_json: String,
    pub has_audio: bool,
    pub source_fetched_at: String,
    pub created_at: String,
    #[serde(default = "default_source")]
    pub source: String,
    /// Normalized entry; absent in version 1 backups (Free Dictionary only).
    #[serde(default)]
    pub entry_json: Option<String>,
    /// Present only for words on the study list.
    #[serde(default)]
    pub progress: Option<BackupProgress>,
    #[serde(default)]
    pub attempts: Vec<BackupAttempt>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupProgress {
    pub status: String,
    pub personal_note: Option<String>,
    pub due_at: String,
    pub interval_days: f64,
    pub repetitions: u32,
    pub ease_factor: f64,
    pub lapses: u32,
    pub added_at: String,
    pub last_reviewed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupAttempt {
    pub mode: String,
    pub expected_answer: String,
    pub submitted_answer: Option<String>,
    pub is_correct: bool,
    pub grade: String,
    pub duration_ms: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportSummary {
    pub words_added: usize,
    pub words_updated: usize,
    pub progress_added: usize,
    pub progress_updated: usize,
    pub attempts_added: usize,
}

fn canonical_ts(value: &str) -> Result<String> {
    Ok(to_db(from_db(value)?))
}

fn canonical_opt_ts(value: Option<&str>) -> Result<Option<String>> {
    value.map(canonical_ts).transpose()
}

impl Database {
    pub fn export_backup(&self, now: DateTime<Utc>) -> Result<Backup> {
        let mut words_stmt = self.conn.prepare(
            "SELECT w.id, w.language_code, w.headword_normalized, w.display_word, w.raw_response_json,
                    w.has_audio, w.source_fetched_at, w.created_at,
                    u.word_id IS NOT NULL, u.status, u.personal_note, u.due_at, u.interval_days,
                    u.repetitions, u.ease_factor, u.lapses, u.added_at, u.last_reviewed_at,
                    w.source, w.entry_json
             FROM words w LEFT JOIN user_words u ON u.word_id = w.id
             ORDER BY w.id",
        )?;
        let mut attempts_stmt = self.conn.prepare(
            "SELECT mode, expected_answer, submitted_answer, is_correct, grade, duration_ms, created_at
             FROM practice_attempts WHERE word_id = ?1 ORDER BY created_at, id",
        )?;

        let rows = words_stmt
            .query_map([], |row| {
                let word_id: i64 = row.get(0)?;
                let progress = if row.get::<_, bool>(8)? {
                    Some(BackupProgress {
                        status: row.get(9)?,
                        personal_note: row.get(10)?,
                        due_at: row.get(11)?,
                        interval_days: row.get(12)?,
                        repetitions: row.get(13)?,
                        ease_factor: row.get(14)?,
                        lapses: row.get(15)?,
                        added_at: row.get(16)?,
                        last_reviewed_at: row.get(17)?,
                    })
                } else {
                    None
                };
                Ok((
                    word_id,
                    BackupWord {
                        language_code: row.get(1)?,
                        headword: row.get(2)?,
                        display_word: row.get(3)?,
                        raw_response_json: row.get(4)?,
                        has_audio: row.get(5)?,
                        source_fetched_at: row.get(6)?,
                        created_at: row.get(7)?,
                        source: row.get(18)?,
                        entry_json: row.get(19)?,
                        progress,
                        attempts: Vec::new(),
                    },
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut words = Vec::with_capacity(rows.len());
        for (word_id, mut word) in rows {
            word.attempts = attempts_stmt
                .query_map([word_id], |row| {
                    Ok(BackupAttempt {
                        mode: row.get(0)?,
                        expected_answer: row.get(1)?,
                        submitted_answer: row.get(2)?,
                        is_correct: row.get(3)?,
                        grade: row.get(4)?,
                        duration_ms: row.get(5)?,
                        created_at: row.get(6)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            words.push(word);
        }

        Ok(Backup {
            format: BACKUP_FORMAT.to_string(),
            version: BACKUP_VERSION,
            exported_at: to_db(now),
            words,
        })
    }

    /// Merges a backup into the database in a single transaction. Re-importing
    /// the same backup is idempotent; for conflicting progress the most recently
    /// reviewed copy wins.
    pub fn import_backup(&mut self, backup: &Backup) -> Result<ImportSummary> {
        if backup.format != BACKUP_FORMAT {
            bail!("not a StealthLingo backup (format \"{}\")", backup.format);
        }
        if backup.version > BACKUP_VERSION {
            bail!(
                "backup version {} is newer than this StealthLingo supports ({BACKUP_VERSION})",
                backup.version
            );
        }

        let tx = self.conn.transaction()?;
        let mut summary = ImportSummary::default();
        for word in &backup.words {
            import_word(&tx, word, &mut summary)
                .with_context(|| format!("could not import \"{}\"", word.display_word))?;
        }
        tx.commit()?;
        Ok(summary)
    }
}

fn import_word(tx: &Transaction<'_>, word: &BackupWord, summary: &mut ImportSummary) -> Result<()> {
    let headword = crate::dictionary::normalize_headword(&word.headword);
    if headword.is_empty() {
        bail!("empty headword");
    }
    let source = word.source.trim();
    if source.is_empty() {
        bail!("missing dictionary source");
    }
    decode_cached(
        &word.display_word,
        &word.raw_response_json,
        word.entry_json.as_deref(),
    )
    .map_err(|e| anyhow::anyhow!("invalid dictionary data: {e}"))?;
    let fetched_at = canonical_ts(&word.source_fetched_at)?;
    let created_at = canonical_ts(&word.created_at)?;

    let existing: Option<(i64, String)> = tx
        .query_row(
            "SELECT id, source_fetched_at FROM words WHERE language_code = ?1 AND headword_normalized = ?2",
            params![word.language_code, headword],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let word_id = match existing {
        None => {
            summary.words_added += 1;
            tx.query_row(
                "INSERT INTO words (language_code, headword_normalized, display_word, raw_response_json,
                                    has_audio, source_fetched_at, created_at, source, entry_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) RETURNING id",
                params![
                    word.language_code,
                    headword,
                    word.display_word,
                    word.raw_response_json,
                    word.has_audio,
                    fetched_at,
                    created_at,
                    source,
                    word.entry_json
                ],
                |row| row.get(0),
            )?
        }
        Some((id, existing_fetched)) => {
            if fetched_at > existing_fetched {
                summary.words_updated += 1;
                tx.execute(
                    "UPDATE words SET display_word = ?2, raw_response_json = ?3, has_audio = ?4,
                            source_fetched_at = ?5, source = ?6, entry_json = ?7 WHERE id = ?1",
                    params![
                        id,
                        word.display_word,
                        word.raw_response_json,
                        word.has_audio,
                        fetched_at,
                        source,
                        word.entry_json
                    ],
                )?;
            }
            id
        }
    };

    if let Some(progress) = &word.progress {
        import_progress(tx, word_id, progress, summary)?;
    }
    for attempt in &word.attempts {
        import_attempt(tx, word_id, attempt, summary)?;
    }
    Ok(())
}

fn import_progress(
    tx: &Transaction<'_>,
    word_id: i64,
    progress: &BackupProgress,
    summary: &mut ImportSummary,
) -> Result<()> {
    let status = Status::parse(&progress.status)
        .with_context(|| format!("unknown status \"{}\"", progress.status))?;
    let due_at = canonical_ts(&progress.due_at)?;
    let added_at = canonical_ts(&progress.added_at)?;
    let last_reviewed = canonical_opt_ts(progress.last_reviewed_at.as_deref())?;

    let existing: Option<(Option<String>, Option<String>)> = tx
        .query_row(
            "SELECT last_reviewed_at, personal_note FROM user_words WHERE word_id = ?1",
            [word_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let values = params![
        word_id,
        status.as_str(),
        progress.personal_note,
        due_at,
        progress.interval_days,
        progress.repetitions,
        progress.ease_factor,
        progress.lapses,
        added_at,
        last_reviewed,
    ];
    match existing {
        None => {
            tx.execute(
                "INSERT INTO user_words (word_id, status, personal_note, due_at, interval_days,
                        repetitions, ease_factor, lapses, added_at, last_reviewed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                values,
            )?;
            summary.progress_added += 1;
        }
        Some((existing_reviewed, existing_note)) => {
            if last_reviewed > existing_reviewed {
                tx.execute(
                    "UPDATE user_words SET status = ?2, personal_note = COALESCE(?3, personal_note),
                            due_at = ?4, interval_days = ?5, repetitions = ?6, ease_factor = ?7,
                            lapses = ?8, added_at = MIN(added_at, ?9), last_reviewed_at = ?10
                     WHERE word_id = ?1",
                    values,
                )?;
                summary.progress_updated += 1;
            } else if existing_note.is_none() && progress.personal_note.is_some() {
                tx.execute(
                    "UPDATE user_words SET personal_note = ?2 WHERE word_id = ?1",
                    params![word_id, progress.personal_note],
                )?;
                summary.progress_updated += 1;
            }
        }
    }
    Ok(())
}

fn import_attempt(
    tx: &Transaction<'_>,
    word_id: i64,
    attempt: &BackupAttempt,
    summary: &mut ImportSummary,
) -> Result<()> {
    let mode = PracticeMode::parse(&attempt.mode)
        .with_context(|| format!("unknown practice mode \"{}\"", attempt.mode))?;
    let grade = Grade::parse(&attempt.grade)
        .with_context(|| format!("unknown grade \"{}\"", attempt.grade))?;
    let created_at = canonical_ts(&attempt.created_at)?;
    let inserted = tx.execute(
        "INSERT INTO practice_attempts (word_id, mode, expected_answer, submitted_answer,
                                        is_correct, grade, duration_ms, created_at)
         SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8
         WHERE NOT EXISTS (
            SELECT 1 FROM practice_attempts
            WHERE word_id = ?1 AND mode = ?2 AND expected_answer = ?3
              AND submitted_answer IS ?4 AND created_at = ?8)",
        params![
            word_id,
            mode.as_str(),
            attempt.expected_answer,
            attempt.submitted_answer,
            attempt.is_correct,
            grade.as_str(),
            attempt.duration_ms,
            created_at,
        ],
    )?;
    summary.attempts_added += inserted;
    Ok(())
}
