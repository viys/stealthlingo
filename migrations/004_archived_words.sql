-- Words removed by Ctrl+clicking "Remove" in `lookup`, with their scheduling
-- state. Links can be triggered from outside StealthLingo, so such removals
-- are recoverable: adding the word again restores this row.
CREATE TABLE archived_user_words (
    word_id          INTEGER PRIMARY KEY REFERENCES words (id) ON DELETE CASCADE,
    status           TEXT    NOT NULL,
    personal_note    TEXT,
    due_at           TEXT    NOT NULL,
    interval_days    REAL    NOT NULL,
    repetitions      INTEGER NOT NULL,
    ease_factor      REAL    NOT NULL,
    lapses           INTEGER NOT NULL,
    added_at         TEXT    NOT NULL,
    last_reviewed_at TEXT,
    archived_at      TEXT    NOT NULL
);
