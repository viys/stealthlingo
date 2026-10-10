-- Who put a word on the study list ('user' or 'mcp', an AI agent connected
-- through `stealthlingo mcp`) and the agent's reason for suggesting it.
ALTER TABLE user_words ADD COLUMN added_via TEXT NOT NULL DEFAULT 'user';
ALTER TABLE user_words ADD COLUMN added_reason TEXT;
ALTER TABLE archived_user_words ADD COLUMN added_via TEXT NOT NULL DEFAULT 'user';
ALTER TABLE archived_user_words ADD COLUMN added_reason TEXT;

-- Words the user removed. Agents may not add them again; the user can.
CREATE TABLE dismissed_words (
    word_id      INTEGER PRIMARY KEY REFERENCES words (id) ON DELETE CASCADE,
    dismissed_at TEXT    NOT NULL
);

INSERT INTO dismissed_words (word_id, dismissed_at)
SELECT word_id, archived_at FROM archived_user_words;

-- One row per word an agent added, for the daily `mcp_daily_add_limit`.
-- Rows stay when the word is removed, so removing does not refund the quota.
CREATE TABLE mcp_add_log (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id    INTEGER NOT NULL REFERENCES words (id) ON DELETE CASCADE,
    created_at TEXT    NOT NULL
);

CREATE INDEX idx_mcp_add_log_created ON mcp_add_log (created_at);
