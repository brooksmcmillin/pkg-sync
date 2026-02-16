# pkg-sync

Compare installed Arch Linux packages across machines. Each machine pushes its package list to a central server, which computes what's missing on each side — with glob-based exclusions so you can ignore hardware-specific packages (e.g. nvidia drivers).

## Setup

### Server

Register your machines and start the server:

```bash
cargo build --release

# Register machines (prints a one-time API key for each)
./target/release/pkg-sync-server register --name desktop
./target/release/pkg-sync-server register --name laptop

# Start the server
./target/release/pkg-sync-server serve --bind 0.0.0.0:3000 --database pkg-sync.db
```

### Client

Create `/etc/pkg-sync/config.toml` on each machine (see [config.example.toml](config.example.toml)):

```toml
[server]
url = "https://your-server.example.com"
api_key = "the-generated-key"

[exclude]
patterns = ["nvidia*", "cuda*", "lib32-nvidia*"]
```

Report packages:

```bash
./target/release/pkg-sync-client                    # push to server
./target/release/pkg-sync-client --dry-run           # preview without sending
```

## API

| Endpoint | Method | Auth | Description |
|---|---|---|---|
| `/api/v1/report` | POST | Bearer token | Submit package list + exclusion patterns |
| `/api/v1/diff?a=desktop&b=laptop` | GET | No | Package diff between two machines |
| `/api/v1/machines` | GET | No | List registered machines |

## How Exclusions Work

Each machine defines glob patterns for packages it doesn't care about. When computing a diff, the server combines exclusion patterns from **both** machines — so if either side excludes `nvidia*`, nvidia packages won't appear as "missing" in the diff regardless of which machine has them.

## Project Structure

```
pkg-sync-common/    # Shared API types (serde structs)
pkg-sync-client/    # CLI: collects packages from pacman, pushes to server
pkg-sync-server/    # axum HTTP server with SQLite storage
```
