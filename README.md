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
