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

## Authentication

Access is protected by a single shared password, stored as an argon2 hash in the `KAMMERZ_PASSWORD_HASH` environment variable.

Generate the hash (the password is read from **stdin** — never pass it as an argument, which would leak it into your shell history and `ps`):

```bash
echo -n 'your-password' | target/release/kammerz hash-password
# or, interactively (prompts with echo off):
target/release/kammerz hash-password
```

Put the resulting `$argon2id$…` string in `KAMMERZ_PASSWORD_HASH`.

> If `KAMMERZ_PASSWORD_HASH` is unset, the app runs in **open (LAN-trust) mode** with no authentication and logs a warning at startup. Only do this on a fully trusted LAN; set the hash for anything network-reachable.

## Configuration (`.env`)

Copy `.env.example` to `.env` and fill it in:

```bash
DATABASE_URL=sqlite:/opt/kammerz/data/kammerz.db?mode=rwc
KAMMERZ_PASSWORD_HASH=$argon2id$v=19$...    # from `kammerz hash-password`
SECURE_COOKIES=false   # set true only when served over HTTPS (behind a TLS reverse proxy)
PORT=3002
ANTHROPIC_API_KEY=     # optional; overrides the claude_api_key settings row for AI import
```

## Deployment (systemd)

On the server (Linux), as a dedicated `kammerz` user:

```bash
sudo install -d -o kammerz -g kammerz /opt/kammerz /opt/kammerz/data
sudo install -o kammerz -g kammerz target/release/kammerz /opt/kammerz/kammerz
sudo install -o kammerz -g kammerz .env /opt/kammerz/.env       # your filled-in .env
sudo cp deploy/kammerz.service /etc/systemd/system/kammerz.service
sudo systemctl daemon-reload
sudo systemctl enable --now kammerz
```

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

A plain file copy is only safe with the service stopped:

```bash
sudo systemctl stop kammerz
cp /opt/kammerz/data/kammerz.db /path/to/backups/
sudo systemctl start kammerz
```

(A clean shutdown checkpoints the WAL, so copying just `kammerz.db` is sufficient.)

### Restoring

Stop the service, replace the database, and remove any stale WAL/SHM sidecar files before starting again:

```bash
sudo systemctl stop kammerz
sudo install -o kammerz -g kammerz backup.db /opt/kammerz/data/kammerz.db
sudo rm -f /opt/kammerz/data/kammerz.db-wal /opt/kammerz/data/kammerz.db-shm
sudo systemctl start kammerz
```

Snapshots from `/api/backup` or `VACUUM INTO` have no sidecar files of their own — they restore as-is.

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
