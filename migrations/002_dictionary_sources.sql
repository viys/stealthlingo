-- Multiple dictionary sources: remember where each cached entry came from and
-- store the normalized entry so it no longer depends on one API's JSON format.
-- Rows without entry_json are Free Dictionary responses from schema version 1.
ALTER TABLE words ADD COLUMN source TEXT NOT NULL DEFAULT 'free-dictionary';
ALTER TABLE words ADD COLUMN entry_json TEXT;
