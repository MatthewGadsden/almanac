-- Entries become items: a note is now one kind among several (film, ...).
-- Kind-specific fields live in `data`; `source` + `external_id` identify imported items
-- so re-syncing a connector updates them instead of duplicating them.
ALTER TABLE entries RENAME TO items;

ALTER TABLE items ADD COLUMN kind TEXT NOT NULL DEFAULT 'note';
ALTER TABLE items ALTER COLUMN kind DROP DEFAULT;

-- Only notes require a topic (enforced by the API); films and imports may have none
ALTER TABLE items ALTER COLUMN topic DROP NOT NULL;

ALTER TABLE items ADD COLUMN data JSONB NOT NULL DEFAULT '{}';

-- When the thing happened (watch date, activity date), as opposed to when it was logged
ALTER TABLE items ADD COLUMN occurred_at timestamptz;
UPDATE items SET occurred_at = created_at;
ALTER TABLE items ALTER COLUMN occurred_at SET NOT NULL;

ALTER TABLE items ADD COLUMN visibility TEXT NOT NULL DEFAULT 'public'
	CHECK (visibility IN ('public', 'private'));

ALTER TABLE items ADD COLUMN source TEXT NOT NULL DEFAULT 'manual';
ALTER TABLE items ADD COLUMN external_id TEXT;
ALTER TABLE items ADD CONSTRAINT ux_items_source_external_id UNIQUE (source, external_id);

CREATE INDEX ix_items_kind_occurred_at ON items (kind, occurred_at DESC);
