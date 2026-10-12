-- OAuth tokens (e.g. Strava's) and each provider's sync position. Neither is ever returned by
-- the API: credentials are secrets, and the cursor is internal.
ALTER TABLE connections ADD COLUMN credentials JSONB;
ALTER TABLE connections ADD COLUMN sync_cursor JSONB;
