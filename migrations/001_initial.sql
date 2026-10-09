-- Dictionary cache: one row per looked-up headword.
CREATE TABLE words (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    language_code       TEXT    NOT NULL DEFAULT 'en',
    headword_normalized TEXT    NOT NULL,
    display_word        TEXT    NOT NULL,
    raw_response_json   TEXT    NOT NULL,
    has_audio           INTEGER NOT NULL DEFAULT 0,
    source_fetched_at   TEXT    NOT NULL,
    created_at          TEXT    NOT NULL,
    UNIQUE (language_code, headword_normalized)
);

-- The user's study list and per-word scheduling state.
CREATE TABLE user_words (
    word_id          INTEGER PRIMARY KEY REFERENCES words (id) ON DELETE CASCADE,
    status           TEXT    NOT NULL DEFAULT 'new'
                     CHECK (status IN ('new', 'learning', 'review', 'mastered')),
    personal_note    TEXT,
    due_at           TEXT    NOT NULL,
    interval_days    REAL    NOT NULL DEFAULT 0,
    repetitions      INTEGER NOT NULL DEFAULT 0,
    ease_factor      REAL    NOT NULL DEFAULT 2.5,
    lapses           INTEGER NOT NULL DEFAULT 0,
    added_at         TEXT    NOT NULL,
    last_reviewed_at TEXT
);

CREATE INDEX idx_user_words_due ON user_words (due_at);

-- Every submitted answer. Correctness and the scheduling grade are stored separately.
CREATE TABLE practice_attempts (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id          INTEGER NOT NULL REFERENCES words (id) ON DELETE CASCADE,
    mode             TEXT    NOT NULL
                     CHECK (mode IN ('memory', 'spelling', 'listening_spelling')),
    expected_answer  TEXT    NOT NULL,
    submitted_answer TEXT,
    is_correct       INTEGER NOT NULL,
    grade            TEXT    NOT NULL CHECK (grade IN ('again', 'hard', 'good', 'easy')),
    duration_ms      INTEGER,
    created_at       TEXT    NOT NULL
);

CREATE INDEX idx_attempts_created ON practice_attempts (created_at);
CREATE INDEX idx_attempts_word ON practice_attempts (word_id);
