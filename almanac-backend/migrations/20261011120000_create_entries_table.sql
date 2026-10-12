CREATE TABLE entries(
	id uuid NOT NULL,
	PRIMARY KEY (id),
	title TEXT NOT NULL,
	body TEXT NOT NULL,
	created_at timestamptz NOT NULL,
	updated_at timestamptz NOT NULL
);
