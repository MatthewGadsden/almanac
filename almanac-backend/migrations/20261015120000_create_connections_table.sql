-- Services that items are imported from (Letterboxd, later Strava, ...). One row per provider.
-- Imported items live in `items`, identified by (source, external_id).
CREATE TABLE connections(
	provider TEXT NOT NULL,
	PRIMARY KEY (provider),
	-- Provider-specific, e.g. {"username": "..."} for Letterboxd
	settings JSONB NOT NULL,
	-- Visibility given to newly imported items
	import_visibility TEXT NOT NULL DEFAULT 'public'
		CHECK (import_visibility IN ('public', 'private')),
	-- Last successful sync and what it did
	last_synced_at timestamptz,
	last_sync_summary JSONB,
	-- Why the most recent sync failed; cleared by the next successful one
	last_error TEXT,
	created_at timestamptz NOT NULL,
	updated_at timestamptz NOT NULL
);
