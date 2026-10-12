<p align="center">
  <img alt="Pixel art sloth" src="https://raw.githubusercontent.com/MatthewGadsden/MatthewGadsden/main/images/sloth.png" width="84" />
</p>
<h1 align="center">
  Almanac
</h1>

<p align="center">A personal digital garden for notes, film reviews and, eventually, every other log worth keeping.</p>

<div align="center">
  <img src="https://img.shields.io/badge/Rust-000?logo=rust&logoColor=fff&style=for-the-badge" alt="Rust Badge">
  <img src="https://img.shields.io/badge/Actix-000?logo=actix&logoColor=fff&style=for-the-badge" alt="Actix Badge">
  <img src="https://img.shields.io/badge/PostgreSQL-4169E1?logo=postgresql&logoColor=fff&style=for-the-badge" alt="PostgreSQL Badge">
  <img src="https://img.shields.io/badge/React-20232A?logo=react&logoColor=61DAFB&style=for-the-badge" alt="React Badge">
  <img src="https://img.shields.io/badge/TypeScript-3178C6?logo=typescript&logoColor=fff&style=for-the-badge" alt="TypeScript Badge">
  <img src="https://img.shields.io/badge/Docker-2496ED?logo=docker&logoColor=fff&style=for-the-badge" alt="Docker Badge">
</div>

<div align="center">
  <img alt="GitHub Actions Workflow Status" src="https://img.shields.io/github/actions/workflow/status/MatthewGadsden/almanac/general-ci.yaml">
  <img alt="GitHub Issues or Pull Requests" src="https://img.shields.io/github/issues-pr/MatthewGadsden/almanac">
</div>

## What it is

Almanac is a [digital garden](https://garden.bradwoods.io/): a place where things are **planted**, **tended** over time, and occasionally **pruned**. Everything in it is an *item* of some kind:

- **Notes:** HTML pages grouped by topic. Each one can carry its own `<style>` block, scoped to that note.
- **Films:** a Letterboxd-style diary with ratings in half stars, liked and rewatch markers, posters and reviews.
- **More to come:** fitness activities, books and other logs, imported from services like Letterboxd and Strava (see the [roadmap](#roadmap)).

Anyone can read the public garden. Writing happens behind an admin login at `/admin`, which nothing on the site links to. Items can also be marked **private**, so only the admin sees them.

## Project structure

| Folder | What's in it |
| --- | --- |
| [`almanac-backend/`](almanac-backend) | Rust API (Actix Web, SQLx, PostgreSQL): items, admin sessions, migrations and integration tests |
| [`almanac-frontend/`](almanac-frontend) | React 19 + TypeScript + Vite UI, using the Tokyo Snow theme from my portfolio |
| `.github/workflows/` | CI: backend tests, rustfmt, clippy and coverage, plus frontend lint and build |

Each folder has its own README with more detail.

## Getting started

You'll need Rust (stable), [Bun](https://bun.sh), Docker, `psql`, and `sqlx-cli`:

```sh
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

**1. Start the database and API**

```sh
cd almanac-backend
./scripts/init_db.sh                       # Postgres in Docker on port 5434, then migrations
cargo run --bin create_admin -- <username> # create your admin account (prompts for a password)
cargo run                                  # API on http://127.0.0.1:8080
```

**2. Start the frontend** (in a second terminal)

```sh
cd almanac-frontend
bun install
bun run dev                                # http://localhost:5173, with /api proxied to the backend
```

Then open http://localhost:5173, or go to http://localhost:5173/admin to log in and plant your first note.

## Development

```sh
# Backend (from almanac-backend/). Needs the database running, because SQLx checks queries at compile time
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings

# Frontend (from almanac-frontend/)
bun run lint
bun run build
```

Settings live in `almanac-backend/configuration.yaml`, and any of them can be overridden with `APP_` environment variables (for example `APP_SESSION_KEY`, or `APP_DATABASE__PASSWORD`). The session key in the file is for local development only.

## Roadmap

- [x] Notes with HTML bodies, topics and a garden-style index
- [x] Admin login, plus private items
- [x] Items with kinds, and films as the first new kind (diary, reviews, ratings)
- [ ] Connector framework, starting with a **Letterboxd** RSS import
- [ ] **Strava** activities (kept private)
- [ ] Links between items (a note that references a film or a run)
- [ ] More kinds and connectors (books, music, …)
