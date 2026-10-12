# Almanac frontend

A digital-garden style UI for [almanac-backend](../almanac-backend): a topic index of notes, latest updates, and pages to plant (create), read, tend (edit) and prune (delete) notes. Built with React 19, TypeScript, React Router and Vite, using the Tokyo Snow theme from my portfolio.

## Getting started

All commands below are run from the `almanac-frontend/` folder. The backend must be running (see its README: `./scripts/init_db.sh`, then `cargo run`).

```sh
bun install
bun run dev       # http://localhost:5173, with /api proxied to the backend on 127.0.0.1:8080
bun run lint
bun run build     # type-check, then build to dist/
```

To point a build at a different API, set `VITE_API_URL` (for example `VITE_API_URL=https://api.example.com bun run build`). The backend has no CORS support yet, so that only works if both are served from the same origin.
