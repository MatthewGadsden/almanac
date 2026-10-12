# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Layout

Two independent projects share one repo:

- `almanac-backend/`: a Rust CRUD API (actix-web + sqlx/Postgres) for notes ("entries": title, topic, body, timestamps).
- `almanac-frontend/`: a React 19 + TypeScript + Vite UI styled as a digital garden (layout modelled on garden.bradwoods.io, look taken from the `~/source/portfolio` project).

Run each project's commands from its own folder. The single CI workflow (`.github/workflows/general-ci.yaml`) defaults `working-directory` to `almanac-backend`, caches `almanac-backend/target`, and its `frontend` job overrides the directory to `almanac-frontend`. Keep these in sync if a folder moves.

## Commands

Backend (`almanac-backend/`):

```sh
./scripts/init_db.sh                    # start Postgres in Docker (port 5434) and run migrations
SKIP_DOCKER=true ./scripts/init_db.sh   # migrate an already-running Postgres (what CI does)
cargo run                               # serve on 127.0.0.1:8080 (configuration.yaml)
cargo test                              # all tests
cargo test --test api <name_substring>  # single test, e.g. `cargo test --test api delete_entry`
cargo fmt --check
cargo clippy -- -D warnings             # CI fails on any warning
sqlx migrate add <name>                 # new migration in migrations/
cargo run --bin create_admin -- <name>  # create an admin (or reset its password); prompts on the TTY
```

Any setting can be overridden with `APP_` env vars, using `__` for nesting: `APP_APPLICATION_PORT=18081`, `APP_DATABASE__DATABASE_NAME=other`. `configuration.rs` must keep `.prefix_separator("_")`, because without it the crate expects `APP__…` and silently ignores these variables.

`sqlx` (sqlx-cli) is installed to `~/.cargo/bin`.

Frontend (`almanac-frontend/`, Bun is the package manager):

```sh
bun install
bun run dev      # Vite on :5173; proxies /api/* to the backend on 127.0.0.1:8080
bun run lint     # eslint flat config (ts-eslint, react-hooks v7, react-refresh)
bun run build    # tsc -b then vite build; CI runs lint + build
```

There is no frontend test suite.

## Things that aren't obvious

- **Building the backend needs a live, migrated database.** `sqlx::query!`/`query_as!` check SQL at compile time against `DATABASE_URL` from `.env`. `cargo build`, `clippy` and `test` all fail if Postgres isn't up or a new migration hasn't been run (`sqlx migrate run`). There is no `.sqlx/` offline cache.
- **Never edit an applied migration**; add a new one. sqlx checksums migrations and refuses to run if an applied file changed.
- **Backend tests create a throwaway database each.** `tests/api/helpers.rs::spawn_app` binds a random port, creates a database named with a fresh UUID, runs `sqlx::migrate!`, and starts the server on it. Every test must end with `app.cleanup().await`, which drops that database. If a test panics first, its database is left behind.
- **Integration tests are one binary:** `tests/api/main.rs` declares the modules, so the test target is `api`.
- **Rust formatting uses hard tabs** (`.rustfmt.toml`) and edition 2024 import ordering. Frontend TS/TSX also uses tabs, single quotes and no semicolons.
- Postgres uses port **5434**, not 5432 or 5433, to avoid clashing with other local projects.
- **The backend has no CORS.** The frontend talks to it only through Vite's dev proxy (`vite.config.ts`, which strips the `/api` prefix). `VITE_API_URL` overrides the base URL, but a cross-origin deployment would need CORS added to the backend first.

## Backend architecture

Request flow: `main.rs` loads `configuration.yaml` (`configuration::get_configuration`), connects a `PgPool`, and calls `startup::run(listener, pool)`. `run` builds the actix `App` and shares the pool as `web::Data<PgPool>`. Tests call the same `run`, so routing changes in `startup.rs` apply to both.

- `src/domain/`: data types. `Entry` mirrors the table row and is the JSON response (also deserialized in tests). `EntryPayload` is the POST/PUT body, with `validate()` (title and topic must not be blank).
- `src/routes/`: handlers, re-exported through `routes/mod.rs` with `pub use`. Handlers return `Result<HttpResponse, ApiError>` and use `?` on sqlx calls.
- `src/error.rs`: `ApiError` maps errors to responses: `NotFound`→404, `Validation(msg)`→400 with `{"error": msg}`, `Database`→500 (logged, body empty). `From<sqlx::Error>` makes `?` work. An invalid UUID in the path or a malformed JSON body is rejected by actix's extractors (404/400, plain-text body) before the handler runs.

- **Auth:** there's a single kind of user, the admin, stored in the `users` table with an Argon2id hash. `POST /admin/login` stores the user's id and username in the session. The session lives entirely in an encrypted cookie (actix-session `CookieSessionStore`, key from `session_key`); nothing is stored server-side, so logging out just clears the cookie. `auth::AdminUser` is an extractor: adding it as a handler argument makes the route return 401 without a session, and it's on every entry write. Reads stay public. Login verifies against a dummy hash when the username doesn't exist, so timing doesn't reveal which usernames exist.
- Tests: `spawn_app` stores a random test admin (with cheap Argon2 params for speed) and uses a cookie-keeping client. Call `app.login_admin().await` before any entry write; `tests/api/entries.rs` uses `spawn_admin_app()` for that.

Adding a resource touches: a migration → a domain type → handlers in `routes/` (plus `pub use` in `routes/mod.rs`) → route registration in `startup.rs` → request helpers on `TestApp` in `tests/api/helpers.rs` and a new test module listed in `tests/api/main.rs`.

## Frontend architecture

- `src/api.ts` is the only code that calls `fetch`. It mirrors the backend's `Entry`/`EntryPayload` types, so a backend field change must be made here too. Errors become `ApiError`, using the backend's `{"error"}` message when present.
- `src/entries/`: `EntriesProvider` loads the full entry list once (`GET /entries`) and holds it in memory. Create, update and delete call the API, then patch that list; nothing is refetched. Every page and the sidebar index read from `useEntries()` rather than fetching. The context and the hook live in `EntriesContext.ts`, separate from the provider component, to satisfy `react-refresh/only-export-components`.
- Routes in `App.tsx` use two layout routes. `GardenLayout` (sidebar with the topic index pinned to the left, plus a wide content area) holds browsing pages: home (latest updates + all notes) and topic. `ReadingLayout` (slim header, single centred column, no sidebar) holds a note (with "Where to next?" siblings), the create/edit form, and the not-found page. Pages derive everything from the entry list. Pages show `<LoadState />` until the list is `ready`. Helpers for grouping, sorting, dates and paths are in `src/notes.ts`. "Tended" means `updated_at !== created_at`, which relies on the backend setting both to the same value on insert.
- **Admin:** `src/auth/AuthProvider.tsx` exposes `useAuth()` (`admin`, `login`, `logout`). The admin area is `/admin` (login form, then a dashboard), and nothing on the public pages links to it. Plant, Tend and Prune controls, and the Admin nav link, only render when `admin` is set. `/notes/new` and `/notes/:id/edit` show the not-found page to visitors, so they don't reveal that a login exists. To keep visitors from even calling `/admin/me`, the provider only checks the session when a localStorage hint says this browser has logged in before. That hint is a convenience, not security; the backend enforces everything.
- **Note bodies are HTML.** `src/noteHtml.ts` sanitises them with DOMPurify, keeping `<style>` and `style` attributes but stripping scripts, event handlers and `javascript:` URLs. A body with no tags is shown as plain-text paragraphs, so older notes still work. `NoteBody` renders the result into a shadow root, with `src/styling/note-body.css` (imported `?inline`) as base typography. That keeps a note's own CSS inside the note; theme custom properties still inherit in. Excerpts use `plainText()`, so tags and `<style>` content don't show up on cards. The backend stores the body as opaque text and does no HTML handling.
- Styling: `src/styling/tokyo_snow.css` is the theme (a copy of the portfolio's, plus a few form/danger tokens). `index.css` holds layout and components and should use only the semantic tokens, never palette colours. The look comes from the portfolio, but the layout follows the garden: the garden sidebar is fixed to the left edge of the screen, not centred, and the content fills the remaining width (capped by `--content-max`). Below 820px everything stacks into blurred bands.
