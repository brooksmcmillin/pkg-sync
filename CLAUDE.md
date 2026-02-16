# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
cargo build                          # build entire workspace
cargo run --bin pkg-sync-server      # run server
cargo run --bin pkg-sync-client      # run client
cargo test                           # run all tests (none yet)
```

The client requires `pacman` (Arch Linux). Use `--dry-run` to test without a server:
```bash
cargo run --bin pkg-sync-client -- --config config.example.toml --dry-run
```

Server subcommands:
```bash
pkg-sync-server register --name <machine> --database pkg-sync.db
pkg-sync-server serve --bind 0.0.0.0:3000 --database pkg-sync.db
```

## Architecture

Cargo workspace with three crates:

- **pkg-sync-common** — Shared serde types for the JSON API (`PackageReport`, `DiffResponse`, `PackageInfo`, etc.). Both client and server depend on this.
- **pkg-sync-client** — CLI that shells out to `pacman -Qe` / `pacman -Qqm` to collect explicitly-installed packages, classifies them as repo vs AUR, reads a TOML config for server URL + API key + exclusion glob patterns, and POSTs a `PackageReport` to the server.
- **pkg-sync-server** — axum HTTP server backed by SQLite (via sqlx, runtime queries not compile-time macros). Machines authenticate with `Authorization: Bearer <key>` validated by an axum `FromRequestParts` extractor (`auth.rs`). Schema migrations are inline `CREATE TABLE IF NOT EXISTS` in `db.rs`.

## Key Design Decisions

- **Exclusions are server-side**: Clients send their full package list plus glob exclusion patterns. The diff endpoint combines exclusion patterns from *both* machines before computing set differences, so excluded packages never appear as "missing" regardless of which machine has them.
- **Package reports are full replacements**: `submit_report` does DELETE + INSERT in a transaction — no incremental updates.
- **API keys are SHA-256 hashed** (not bcrypt/argon2) since these are machine-to-machine keys, not user passwords.
- **No compile-time SQL checking**: Uses `sqlx::query()` / `sqlx::query_as()` with runtime string queries to avoid needing a DATABASE_URL at build time.

## API Endpoints

- `POST /api/v1/report` — authenticated; accepts `PackageReport` JSON
- `GET /api/v1/diff?a=<name>&b=<name>` — unauthenticated; returns symmetric diff
- `GET /api/v1/machines` — unauthenticated; lists machines with package counts

## Client Config

TOML file (default `/etc/pkg-sync/config.toml`), see `config.example.toml`:
```toml
[server]
url = "https://your-server.example.com"
api_key = "your-api-key-here"

[exclude]
patterns = ["nvidia*", "cuda*"]
```
