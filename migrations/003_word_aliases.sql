-- What the user typed when a dictionary answered with a different headword
-- (e.g. "ran" -> "run"), so later lookups of the typed form hit the cache.
CREATE TABLE word_aliases (
    language_code TEXT    NOT NULL DEFAULT 'en',
    alias         TEXT    NOT NULL,
    word_id       INTEGER NOT NULL REFERENCES words (id) ON DELETE CASCADE,
    PRIMARY KEY (language_code, alias)
);
