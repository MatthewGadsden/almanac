# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Layout

Almanac is a personal digital garden: one place for notes, film reviews and, eventually, other logs (fitness, books, …) pulled in from services like Letterboxd and Strava. Two independent projects share the repo:

- `almanac-backend/`: a Rust API (actix-web + sqlx/Postgres) for **items**. Every logged thing is an item with a `kind`.
- `almanac-frontend/`: React 19 + TypeScript + Vite. The layout is modelled on garden.bradwoods.io; the look (Tokyo Snow theme, type, cards, pills) comes from the `~/source/portfolio` project.

Run each project's commands from its own folder. The single CI workflow (`.github/workflows/general-ci.yaml`) defaults `working-directory` to `almanac-backend`, caches `almanac-backend/target`, and its `frontend` job overrides the directory to `almanac-frontend`. Keep these in sync if a folder moves.

## Commands

Backend (`almanac-backend/`; `sqlx` (sqlx-cli) is in `~/.cargo/bin`):

```sh
./scripts/init_db.sh                    # start Postgres in Docker (port 5434) and run migrations
SKIP_DOCKER=true ./scripts/init_db.sh   # migrate an already-running Postgres (what CI does)
cargo run                               # serve on 127.0.0.1:8080 (configuration.yaml)
cargo test                              # all tests
cargo test --test api <name_substring>  # single test, e.g. `cargo test --test api create_film`
cargo fmt --check
cargo clippy --all-targets -- -D warnings   # what CI runs; any warning, including in tests, fails
sqlx migrate add <name>                 # new migration in migrations/
sqlx migrate run                        # apply pending migrations to DATABASE_URL (.env: the dev DB)
cargo run --bin create_admin -- <name>  # create an admin (or reset its password); prompts on the TTY
```

Any setting can be overridden with `APP_` env vars, using `__` for nesting: `APP_APPLICATION_PORT=18081`, `APP_DATABASE__DATABASE_NAME=other`, `APP_SESSION_KEY=…`, `APP_SECURE_COOKIES=true`. `configuration.rs` must keep `.prefix_separator("_")`, because without it the crate expects `APP__…` and silently ignores these variables.

Frontend (`almanac-frontend/`, Bun is the package manager):

```sh
bun install
bun run dev      # Vite on :5173; proxies /api/* to the backend on 127.0.0.1:8080
bun run lint     # eslint flat config (ts-eslint, react-hooks v7, react-refresh)
bun run build    # tsc -b then vite build; CI runs lint + build
```

There is no frontend test suite.

## Working in this repo

- **The dev database (`almanac` on port 5434) holds the user's real data.** Ask before running migrations against it, running `init_db.sh`, or anything that starts Docker. The user usually has their own `cargo run` and `bun run dev` going, so don't restart them.
- **Verify against throwaway databases instead.** sqlx's compile-time checks read `DATABASE_URL` from the environment before `.env`, so `DATABASE_URL=postgres://postgres:postgres@localhost:5434/almanac_build` with `sqlx database create && sqlx migrate run` lets you build, run clippy and test new migrations without touching the dev DB. For end-to-end checks, run a second backend with `APP_APPLICATION_PORT=18081 APP_DATABASE__DATABASE_NAME=almanac_e2e`. Afterwards, confirm with `psql` that anything you created landed in the throwaway DB, then drop the throwaway databases.
- **UI changes:** the user wants screenshots. Playwright plus Chromium can live in the session scratchpad (not the project). Point the browser at the running Vite server and either intercept `**/api/items` with sample JSON, or rewrite `/api/*` to the e2e backend.
- **API changes reach the user's Vite server immediately**, while their backend keeps running the old build until they migrate and restart. When an API change is breaking, tell them what to run.

## Things that aren't obvious

- **Building the backend needs a live, migrated database.** `sqlx::query!`/`query_as!` check SQL at compile time against `DATABASE_URL`. `cargo build`, `clippy` and `test` all fail if Postgres isn't up or a new migration hasn't been run. There is no `.sqlx/` offline cache.
- **Never edit an applied migration**; add a new one. sqlx checksums migrations and refuses to run if an applied file changed.
- **Backend tests create a throwaway database each.** `tests/api/helpers.rs::spawn_app` binds a random port, creates a database named with a fresh UUID, runs `sqlx::migrate!`, stores a random test admin, and starts the server on it. Every test must end with `app.cleanup().await`, which drops that database. If a test panics first, its database is left behind.
- **Integration tests are one binary:** `tests/api/main.rs` declares the modules, so the test target is `api`.
- **Formatting:** Rust uses hard tabs (`.rustfmt.toml`) and edition 2024 import ordering. Frontend TS/TSX uses tabs, single quotes and no semicolons.
- Postgres uses port **5434**, not 5432 or 5433, to avoid clashing with other local projects.
- **The backend has no CORS.** The frontend talks to it only through Vite's dev proxy (`vite.config.ts`, which strips the `/api` prefix), and the session cookie relies on that same origin. `VITE_API_URL` overrides the base URL, but a cross-origin deployment would need CORS (and cookie settings) changed first.

## Backend architecture

Request flow: `main.rs` loads `configuration.yaml` (`configuration::get_configuration`), connects a `PgPool`, and calls `startup::run(listener, pool, SessionSettings::from_settings(&config))`. `run` builds the actix `App` with the session middleware and shares the pool as `web::Data<PgPool>`. Tests call the same `run`, so routing changes in `startup.rs` apply to both.

- **Data model:** everything is a row in `items`, with a `kind` (`ItemKind`: `note`, `film`).
  - `data` (JSONB) holds fields specific to a kind: `{}` for notes, `FilmData` for films.
  - `occurred_at` is when the thing happened (the watch date); `created_at` is when it was logged.
  - `topic` is required for notes and optional for other kinds.
  - `visibility` is `public` or `private`. Private items are only returned when an admin session is present, and visitors get a 404 for them.
  - `source` + `external_id` (unique together) identify imported items, so connectors can upsert instead of duplicating. Everything is `manual` today.
- `src/domain/`: `Item` (the JSON response; tests deserialise it too), `ItemPayload` (POST/PUT body) and `validate()`, which returns a `ValidItem` with `data` normalised per kind. `FilmData::parse` validates film fields with `deny_unknown_fields`: rating 0.5–5 in half stars, year 1870–2100, http(s) poster URL.
- `src/routes/items.rs`: handlers query into a private `ItemRow` (`kind`/`visibility` as `String`) and convert with `TryFrom<ItemRow> for Item`. sqlx enum derives aren't used for TEXT columns. Reads take `Option<AdminUser>` to decide whether private items are included. On update, `occurred_at`/`visibility` use `COALESCE`, so leaving them out keeps the current value. Lists are ordered by `occurred_at DESC`.
- `src/error.rs`: `ApiError` maps errors to responses:
  - `NotFound` → 404
  - `Validation(msg)` → 400 with `{"error": msg}`
  - `Unauthorized` (no session) and `InvalidCredentials` → 401 with `{"error": …}`
  - `Database` / `Unexpected` → 500, logged, with an empty body

  `From<sqlx::Error>` makes `?` work. An invalid UUID in the path or a malformed JSON body is rejected by actix's extractors (404/400, plain-text body) before the handler runs.
- **Auth:** there's a single kind of user, the admin, stored in `users` with an Argon2id hash.
  - `POST /admin/login` stores the user's id and username in the session. Other admin routes: `POST /admin/logout`, `GET /admin/me`.
  - The session lives entirely in an encrypted cookie: actix-session `CookieSessionStore`, key from `session_key` (64+ characters), `SameSite=Strict`, 7-day TTL, and `Secure` when `secure_cookies` is set. Nothing is stored server-side, so logging out just clears the cookie.
  - `auth::AdminUser` is an extractor. Adding it as a handler argument makes the route return 401 without a session; it's on every item write. `Option<AdminUser>` never fails, which is how reads include private items for the admin.
  - Login verifies against a dummy hash when the username doesn't exist, so timing doesn't reveal which usernames exist.
- **Tests:** `spawn_app` uses a cookie-keeping client and a test admin with cheap Argon2 params. Call `app.login_admin().await` before any item write; `spawn_admin_app()` in `tests/api/items.rs` does that, and `films.rs` reuses it.
- Handlers are re-exported through `routes/mod.rs` with `pub use`. Handlers return `Result<HttpResponse, ApiError>` and use `?` on sqlx calls.

**Adding a kind** (preferred over a new resource for anything that's "a thing I logged"):
1. Add a variant to `ItemKind` (and to its `as_str`/`parse`).
2. Add a data struct with `parse`/`to_value`, plus a match arm in `ItemPayload::validate`.
3. Add tests (see `tests/api/films.rs`).
4. Update the frontend (see below).

No migration is needed unless the kind needs its own index.

**Adding a resource** that isn't an item touches: a migration → a domain type → handlers in `routes/` (plus `pub use` in `routes/mod.rs`) → route registration in `startup.rs` → request helpers on `TestApp` in `tests/api/helpers.rs` and a new test module listed in `tests/api/main.rs`.

## Frontend architecture

- `src/api.ts` is the only code that calls `fetch`. It mirrors the backend's types as a discriminated union (`Item = NoteItem | FilmItem`, keyed on `kind`), so a backend field change must be made here too. Errors become `ApiError`, using the backend's `{"error"}` message when present.
- **State** (`main.tsx` nests `AuthProvider` outside `ItemsProvider`; the order matters):
  - `src/auth/AuthProvider.tsx` exposes `useAuth()` (`admin`, `status`, `login`, `logout`).
  - `src/state/ItemsProvider.tsx` loads every item the viewer may see (`GET /items`) and holds it in memory. It reloads when the viewer changes (login/logout), because the admin also gets private items. A list loaded for a different viewer is treated as still loading, which avoids calling setState synchronously inside an effect.
  - Create, update and delete patch the in-memory list after the API call succeeds; nothing is refetched. Pages and the sidebar read from `useItems()` and derive everything from that list, showing `<LoadState />` until it's `ready`.
  - Contexts and hooks live in `*Context.ts` files, separate from the providers, to satisfy `react-refresh/only-export-components`. Shared constants go in `src/site.ts` for the same reason.
- `src/kinds.ts` holds each kind's label, plural, URL slug (`notes`, `films`) and create verb ("Plant a note", "Log a film"), plus `itemPath`/`editItemPath`/`newItemPath`.
- `src/items.ts` holds the pure helpers: sorting, `groupByTopic`/`groupByMonth`, dates (`toDateInput`/`fromDateInput` store a picked date at midday UTC so it's the same day in any timezone), `excerpt`, `stars()`, and the `isNote`/`isFilm` type guards. "Tended" means `updated_at !== created_at`, which relies on the backend setting both to the same value on insert.
- **Routes** (`App.tsx`) use two layout routes:
  - `GardenLayout` (sidebar with the index pinned to the left edge, plus a wide content area) holds browsing pages:
    - **Home:** latest updates and all notes, with the 4 most recently watched films (newest first) in a right-hand column. The column narrows at 1400px and stacks below "Latest updates" under 1180px.
    - **`/films`:** the diary, grouped by month.
    - **`/topics/:topic`.**
  - `ReadingLayout` (slim header, centred column) holds:
    - `/notes/:id` and `/films/:id`, both `ItemPage` with a header and aside per kind
    - `/<kind>/new` and `/<kind>/:id/edit`, both `ItemFormPage`
    - `/admin` and not-found
- **Components:**
  - `ItemCard` (note card, or a film poster tile)
  - `Film.tsx` (`FilmPoster` with a placeholder when there's no poster URL, `Rating`, `FilmFlags`)
  - `PrivateBadge`, `NoteBody`, `Sidebar`, `LoadState`
- **Adding a kind (frontend):**
  1. Extend `ItemKind` and the union in `api.ts`.
  2. Add an entry to `KINDS`.
  3. Add a card in `ItemCard`, a header and aside in `ItemPage`, and form fields in `ItemFormPage`.
  4. Add routes in `App.tsx`, and anything kind-specific on the home page or sidebar index.
- **Admin:**
  - The admin area is `/admin` (a login form, then a dashboard listing everything), and nothing on the public pages links to it.
  - Create/Tend/Prune controls and the Admin nav link only render when `admin` is set. The `new`/`edit` routes show not-found to visitors, so they don't reveal that a login exists.
  - The provider only checks the session (`/admin/me`) when a localStorage hint says this browser has logged in before, so visitors never call it. The hint is a convenience; the backend enforces everything.
- **Item bodies (notes and film reviews) are HTML:**
  - `src/noteHtml.ts` sanitises them with DOMPurify, keeping `<style>` and `style` attributes but stripping scripts, event handlers and `javascript:` URLs. A body with no tags is shown as plain-text paragraphs.
  - `NoteBody` renders into a shadow root, with `src/styling/note-body.css` (imported `?inline`) as base typography. That keeps an item's own CSS inside it, while theme custom properties still inherit in.
  - Excerpts use `plainText()`, which separates block elements and drops `<style>`.
  - The backend stores bodies as opaque text.
- **Styling:**
  - `src/styling/tokyo_snow.css` is the theme: a copy of the portfolio's, plus form/danger tokens.
  - `index.css` holds layout and components and must use only the semantic tokens, never palette colours.
  - Lists of rows (the diary, Recently watched, the admin list) share one panel background with dividers between rows, rather than outlining each row. Hover effects stay subtle (`--column-tint`).
  - Below 820px everything stacks into blurred bands.

## Direction

The agreed plan:
1. ~~Generalise entries into items~~ (done)
2. ~~Films~~ (done)
3. **Next:** a connector framework (a `connections` table holding credentials, a sync cursor and last-sync time; a sync timer plus "Sync now"; upserts on `source` + `external_id`), starting with a **Letterboxd RSS** connector. Letterboxd has no open API.
4. **Strava:** OAuth, with activities kept **private**. Strava's API terms (as of late 2024) only allow showing a user their own data, and GPS routes can reveal a home location.
5. Links between items, and more kinds and connectors.
