# Kammerz

A film photography catalog — track cameras, lenses, film stocks, rolls, shots, and development records.

Kammerz is a self-hosted web app: a single Rust binary (axum) that serves a SvelteKit single-page app and a JSON API, backed by SQLite. It's designed to run on a home server / NAS, reachable on the LAN and — from the field — over your gateway's VPN. Access is gated by a single shared password.

## Tech stack

axum 0.8 · SvelteKit (Svelte 5 runes) · SeaORM 1.1 + SQLite · tower-sessions + argon2 · rust-embed · Tailwind 4 · Bun.

## Development

Prerequisites: Rust (stable), [Bun](https://bun.sh), and [`just`](https://github.com/casey/just).

```bash
just dev
```

This runs the axum backend on `:3002` and the Vite dev server on `:5273` (which proxies `/api` → `:3002`). Open <http://localhost:5273>. The dev database is `./kammerz.db` (created on first run; override with `DATABASE_URL`).

Run the halves separately with `just dev-backend` / `just dev-frontend`.

Checks:

```bash
just check        # bun run check (svelte-check) + cargo build + cargo test
cargo test -p kammerz   # backend integration tests (in-memory SQLite)
bun run build     # (in frontend/) frontend type/compile check
```

## Production build

```bash
just build
```

This builds the SvelteKit app into `frontend/build`, then `cargo build --release` embeds it into the binary at `target/release/kammerz`. The release binary serves the SPA and the API itself — no separate web server or Node runtime needed.

Note that `just build` targets the **host** you run it on (on a Mac it produces a macOS binary). Deploying to the Linux server is handled by `just deploy`, which cross-compiles automatically (see [Deployment](#deployment-systemd)).

## Authentication

Access is protected by a single shared password, stored as an argon2 hash in the `KAMMERZ_PASSWORD_HASH` environment variable.

Generate the hash (the password is read from **stdin** — never pass it as an argument, which would leak it into your shell history and `ps`):

```bash
echo -n 'your-password' | target/release/kammerz hash-password
# or, interactively (prompts with echo off):
target/release/kammerz hash-password
```

Put the resulting `$argon2id$…` string in `KAMMERZ_PASSWORD_HASH`, **wrapped in single quotes** when it lives in a `.env` file — the hash is full of `$` tokens, and the `.env` loader performs `$VAR` substitution on unquoted values, silently mangling the hash so every login fails. (The app refuses to start on an unparseable hash, so a mangled value is caught immediately.)

> If `KAMMERZ_PASSWORD_HASH` is unset, the app runs in **open (LAN-trust) mode** with no authentication and logs a warning at startup. Only do this on a fully trusted LAN; set the hash for anything network-reachable.

## Configuration (`.env`)

Copy `.env.example` to `.env` and fill it in:

```bash
DATABASE_URL=sqlite:/opt/kammerz/data/kammerz.db?mode=rwc
KAMMERZ_PASSWORD_HASH='$argon2id$v=19$...'  # from `kammerz hash-password` — keep the single quotes!
SECURE_COOKIES=false   # set true only when served over HTTPS (behind a TLS reverse proxy)
PORT=3002
ANTHROPIC_API_KEY=     # optional; overrides the claude_api_key settings row for AI import
```

## Deployment (systemd)

Releases are deployed straight from this repo with `just deploy` — there are no GitHub release artifacts. One-time toolchain setup on the Mac (cross-compiler for the aarch64 DietPi server; the linker is wired up in `.cargo/config.toml`):

```bash
rustup target add aarch64-unknown-linux-gnu
brew install messense/macos-cross-toolchains/aarch64-unknown-linux-gnu
```

First-time setup on the server, as a dedicated `kammerz` user:

```bash
sudo install -d -o kammerz -g kammerz /opt/kammerz /opt/kammerz/data
sudo install -o kammerz -g kammerz .env /opt/kammerz/.env       # your filled-in .env
```

Then every release is one command from the Mac:

```bash
just deploy <user>@<server>
```

This cross-compiles the binary (frontend embedded via rust-embed — a fresh SPA build is part of the recipe), streams it to `/opt/kammerz/kammerz`, installs `deploy/kammerz.service` into `/etc/systemd/system/` (so unit-file edits always propagate), restarts the service, and verifies the deploy by polling `GET /api/health` until it answers with the running version (also logged at startup: `kammerz vX.Y.Z starting`). After the first deploy, enable boot startup once: `ssh <user>@<server> 'sudo systemctl enable kammerz'`.

The provided `deploy/kammerz.service` is hardened (`ProtectSystem=strict`, `ProtectHome`, `PrivateTmp`, `NoNewPrivileges`) and only grants write access to `/opt/kammerz/data`, where the SQLite catalog lives.

## Backups

The database runs in **WAL mode**, so the most recent writes live in `kammerz.db-wal` next to the main file. **Do not back up a running instance with a plain `cp kammerz.db`** — the copy misses everything still in the WAL and can be torn mid-checkpoint. Use one of the safe methods below.

### One-tap: download from the browser (works over the VPN)

While logged in, visit:

```
http://<server>:3002/api/backup
```

The server takes a consistent `VACUUM INTO` snapshot of the live database and downloads it as `kammerz-backup-YYYY-MM-DD.db`. No shell access to the server needed — this works from a phone in the field over the VPN. The endpoint requires authentication like every other API route.

Scripted (e.g. from another machine that pulls backups):

```bash
curl -s -c /tmp/kz-cookies -H 'content-type: application/json' \
    -d '{"password":"your-password"}' http://<server>:3002/api/auth/login
curl -s -b /tmp/kz-cookies -o "kammerz-backup-$(date +%F).db" http://<server>:3002/api/backup
```

### On the server: `sqlite3` while the service runs

`VACUUM INTO` (or `.backup`) is the only safe way to copy the file online:

```bash
sqlite3 /opt/kammerz/data/kammerz.db "VACUUM INTO '/path/to/backups/kammerz-$(date +%F).db'"
```

The output is a complete, WAL-free, single-file snapshot — drop it into a cron job. Note: `VACUUM INTO` refuses to overwrite an existing file, so date-stamp the target.

### Cold copy

A plain file copy is only safe with the service stopped — **and the WAL sidecar must come along**. The server does not checkpoint the WAL on shutdown, so after `systemctl stop` recent writes (potentially the entire database, before the first autocheckpoint) still live in `kammerz.db-wal`. Copying only `kammerz.db` produces a stale or empty backup.

```bash
sudo systemctl stop kammerz
cp /opt/kammerz/data/kammerz.db* /path/to/backups/   # .db + -wal + -shm together
sudo systemctl start kammerz
```

If you want a single-file snapshot instead, fold the WAL into the main file first (this also removes the sidecars):

```bash
sudo systemctl stop kammerz
sudo sqlite3 /opt/kammerz/data/kammerz.db "PRAGMA wal_checkpoint(TRUNCATE);"
cp /opt/kammerz/data/kammerz.db /path/to/backups/
sudo systemctl start kammerz
```

### Restoring

Stop the service, replace the database, and remove any stale WAL/SHM sidecar files before starting again:

```bash
sudo systemctl stop kammerz
sudo install -o kammerz -g kammerz backup.db /opt/kammerz/data/kammerz.db
sudo rm -f /opt/kammerz/data/kammerz.db-wal /opt/kammerz/data/kammerz.db-shm
sudo systemctl start kammerz
```

Snapshots from `/api/backup` or `VACUUM INTO` have no sidecar files of their own — they restore as-is. If you are restoring a cold-copy *set* (`kammerz.db` + `-wal` + `-shm` copied together), restore all three files in place of the `rm` step — the WAL sidecar holds the most recent writes and must not be dropped.

## Field access over VPN

The app stays **LAN-bound** — it binds `0.0.0.0:$PORT` and is not exposed to the public internet. To reach it away from home, connect your phone/laptop to your home network's VPN at the gateway, then browse to the server's LAN address.

This is configured **on the UniFi gateway**, not in Kammerz (see the `../unifi-management` MCP for managing the gateway/VPN). No application code or config is involved in VPN access.

## Carrying over an existing catalog

If you ran the earlier Tauri desktop version, copy its SQLite database to the server's data directory **before the first run** of the web app:

```bash
scp ~/Library/Application\ Support/com.kammerz.app/kammerz.db \
    <server>:/opt/kammerz/data/kammerz.db
```

On boot, `seaql_migrations` is already populated, so `Migrator::up` is a no-op and your existing cameras, lenses, rolls, and shots appear as-is. The only schema addition is the `tower_sessions` table (created automatically by the session store). With no DB copied, the first boot creates a fresh database and seeds the default gear via migrations.
