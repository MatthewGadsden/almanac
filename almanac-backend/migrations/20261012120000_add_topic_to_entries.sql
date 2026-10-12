-- Existing rows are filed under "Misc."; after that every insert must supply a topic.
ALTER TABLE entries ADD COLUMN topic TEXT NOT NULL DEFAULT 'Misc.';
ALTER TABLE entries ALTER COLUMN topic DROP DEFAULT;
