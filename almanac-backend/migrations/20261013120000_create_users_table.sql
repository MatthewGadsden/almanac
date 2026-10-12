-- Admin accounts. Create one with `cargo run --bin create_admin -- <username>`.
CREATE TABLE users(
	id uuid NOT NULL,
	PRIMARY KEY (id),
	username TEXT NOT NULL UNIQUE,
	password_hash TEXT NOT NULL,
	created_at timestamptz NOT NULL
);
