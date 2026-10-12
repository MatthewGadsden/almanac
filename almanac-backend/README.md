<p align="center">
  <a href="https://www.rust-lang.org">
    <img alt="rust logo" src="https://upload.wikimedia.org/wikipedia/commons/d/d5/Rust_programming_language_black_logo.svg" width="70" />
  </a>
</p>
<h1 align="center">
  Almanac
</h1>

<div align="center">
  <img src="https://img.shields.io/badge/Rust-000?logo=rust&logoColor=fff&style=for-the-badge" alt="Rust Badge">
  <img src="https://img.shields.io/badge/Docker-2496ED?logo=docker&logoColor=fff&style=for-the-badge" alt="Docker Badge">
  <img src="https://img.shields.io/badge/Actix-000?logo=actix&logoColor=fff&style=for-the-badge" alt="Actix Badge">
</div>

<div align="center">
  <img alt="GitHub Issues or Pull Requests" src="https://img.shields.io/github/issues-pr/MatthewGadsden/almanac">
  <img alt="GitHub Actions Workflow Status" src="https://img.shields.io/github/actions/workflow/status/MatthewGadsden/almanac/general-ci.yaml">
  <img alt="GitHub Tag" src="https://img.shields.io/github/v/tag/MatthewGadsden/almanac">
</div>

A CRUD backend built with Rust, Actix Web, and Postgres (via sqlx).

## Getting started

All commands below are run from the `almanac-backend/` folder.

Prerequisites: Rust (stable), Docker, `psql`, and `sqlx-cli`:

```sh
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

Start Postgres (on port `5434`) and run the migrations:

```sh
./scripts/init_db.sh
```

Create an admin account (prompts for a password, at least 12 characters; re-run to reset it):

```sh
cargo run --bin create_admin -- <username>
```

Then run the API (listens on `127.0.0.1:8080`, see `configuration.yaml`):

```sh
cargo run
```

Run the tests (each test creates and drops its own database):

```sh
cargo test
```

## Endpoints

| Method   | Path             | Description              | Success |
| -------- | ---------------- | ------------------------ | ------- |
| `GET`    | `/health_check`  | Liveness check           | `200`   |
| `GET`    | `/items`         | List items, newest first | `200`   |
| `POST`   | `/items`         | Create an item (admin)   | `201`   |
| `GET`    | `/items/{id}`    | Fetch a single item      | `200`   |
| `PUT`    | `/items/{id}`    | Replace an item (admin)  | `200`   |
| `DELETE` | `/items/{id}`    | Delete an item (admin)   | `204`   |
| `POST`   | `/admin/login`   | Log in, sets the session cookie | `200` |
| `POST`   | `/admin/logout`  | End the session          | `204`   |
| `GET`    | `/admin/me`      | The logged-in admin      | `200`   |

Everything in the garden is an **item** with a `kind` (`note` or `film`). `POST` and `PUT` take:

```json
{
  "kind": "film",
  "title": "Perfect Days",
  "topic": null,
  "body": "<p>Quietly wonderful.</p>",
  "occurred_at": "2026-10-11T12:00:00Z",
  "visibility": "public",
  "data": { "year": 2023, "director": "Wim Wenders", "rating": 4.5, "liked": true, "rewatch": false, "poster_url": "https://..." }
}
```

- `topic` is required for notes and optional for other kinds.
- `data` holds fields specific to a kind. Notes have none. A film's `data` is validated: `rating` must be 0.5 to 5 in half stars, `year` must be between 1870 and 2100, `poster_url` must be http(s), and unknown fields are rejected.
- `occurred_at` (when it happened) defaults to now on create. `visibility` defaults to `public`. On `PUT`, leaving either out keeps the current value.
- Private items are only returned to the logged-in admin. Visitors get `404` for them.
- Unknown ids return `404`. Invalid payloads return `400`.

Endpoints marked (admin) return `401` without a session. `/admin/login` takes `{ "username": "...", "password": "..." }`.

## Configuration

Settings come from `configuration.yaml`, and any of them can be overridden with `APP_` environment variables, using `__` for nesting (e.g. `APP_SESSION_KEY=...`, `APP_DATABASE__PASSWORD=...`). The `session_key` in `configuration.yaml` is for local development only. Set `APP_SESSION_KEY` (at least 64 characters) and `APP_SECURE_COOKIES=true` anywhere else.
