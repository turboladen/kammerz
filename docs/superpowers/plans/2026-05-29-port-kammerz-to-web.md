# Port Kammerz to Axum + SvelteKit Web App — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert the Tauri 2 desktop app into a self-hosted axum + SvelteKit web app (single-password auth, LAN + UniFi-VPN access) while reusing the existing SeaORM/SQLite core unchanged.

**Architecture:** Restructure into a Cargo workspace mirroring chorez: an `entity` crate (SeaORM models), the existing `migration` crate, and a root binary crate (axum server). The server exposes `/api/*` JSON endpoints (one per former Tauri command), guards them with a `tower-sessions` single-password gate, and embeds the SvelteKit build via `rust-embed` with an SPA fallback. The SvelteKit app moves under `frontend/`; its 13 `src/lib/api/*` wrappers swap `invoke()` for a shared `fetch` helper. All routes move under an `(app)/` group behind an auth guard, with a new `/login` page.

**Tech Stack:** Rust, axum 0.8, tower-sessions 0.14 + tower-sessions-sqlx-store, rust-embed 8, argon2, SeaORM 1.1, SQLite, SvelteKit (Svelte 5 runes), adapter-static, Tailwind 4, Bun.

---

## Reference Conventions (read once before starting)

**Target directory layout (after Phase 1):**
```
kammerz/
  Cargo.toml              # [workspace] members: [".", "entity", "migration"]
  src/                    # axum binary crate
    main.rs               # bootstrap: config, db, migrate, sessions, router, serve
    lib.rs                # AppState + module re-exports + create_router()
    config.rs             # AppConfig::from_env()
    db.rs                 # single-conn pool; FK-off migrate, FK-on runtime
    error.rs              # AppError + IntoResponse
    patch.rs              # trim/trim_opt/double_option (moved from src-tauri)
    auth/
      mod.rs              # re-exports
      middleware.rs       # RequireAuth extractor + session helpers
      handlers.rs         # login / logout / me
      password.rs         # argon2 verify
    routes/
      mod.rs              # create_router() merges all sub-routers + health
      cameras.rs lenses.rs lens_mounts.rs film_stocks.rs labs.rs
      rolls.rs shots.rs development.rs search.rs stats.rs settings.rs import.rs
    services/             # MOVED from src-tauri/src/services (imports rewritten)
  entity/
    Cargo.toml
    src/lib.rs            # pub mod camera; ... (was entities/mod.rs)
    src/<entity>.rs       # MOVED from src-tauri/src/entities
  migration/              # MOVED from src-tauri/migration (unchanged contents)
  frontend/               # MOVED SvelteKit app
    package.json svelte.config.js vite.config.ts tsconfig.json .npmrc
    src/ static/ build/
  .env.example
  deploy/kammerz.service
  justfile                # updated recipes
```

**RPC→REST endpoint design:** Every former command maps to one route. Reads use `GET`, creates `POST`, updates `PUT`, deletes `DELETE`. `id` goes in the path; complex payloads in the JSON body. **Responses return the raw value (no `{data}` wrapper)** so frontend wrapper return types stay identical. `void` commands return `204 No Content`. Errors return `{ "error": { "code", "message" } }`.

**Ports & dev flow:** axum binds `0.0.0.0:${PORT:-3001}`. Vite dev server runs on `5173` and proxies `/api` → `http://localhost:3001`. In production the binary serves the embedded `frontend/build` and all `/api/*` itself.

**Verification commands used throughout:**
- Backend compile: `cargo build` (from repo root)
- Backend tests: `cargo test -p kammerz` (or `cargo test` for all crates)
- Frontend compile: `cd frontend && bun run build`
- Frontend type-check: `cd frontend && bun run check`

---

## PHASE 0 — Branch & baseline

### Task 0: Confirm clean baseline on the feature branch

**Files:** none (git only)

- [ ] **Step 1: Verify branch + clean tree**

Run: `git status && git branch --show-current`
Expected: branch `port-to-axum-web`, working tree clean (the spec doc is already committed here).

- [ ] **Step 2: Capture a pre-port build baseline**

Run: `cd src-tauri && cargo build 2>&1 | tail -5; cd ..`
Expected: the current Tauri backend compiles. Record that it builds — this is the behavior we must preserve at the service layer.

- [ ] **Step 3: Snapshot the current DB schema for later parity check**

Run: `sqlite3 ~/Library/Application\ Support/com.kammerz.app/kammerz.db ".schema" > /tmp/kammerz-schema-before.sql 2>/dev/null; wc -l /tmp/kammerz-schema-before.sql || echo "no local DB — fresh install path"`
Expected: either a schema dump (keep it) or confirmation there's no local DB (the port will seed a fresh one via migrations).

---

## PHASE 1 — Workspace restructure (compiles & serves, no business routes yet)

> End state: a root axum binary that connects the DB, runs migrations correctly, serves `GET /api/health` and the (not-yet-moved) SPA fallback. Entity + migration crates split out. Services compile against the new `entity` crate. No `/api` business routes and no auth yet.

### Task 1: Extract the `entity` crate

**Files:**
- Create: `entity/Cargo.toml`
- Create: `entity/src/lib.rs`
- Move: `src-tauri/src/entities/*.rs` → `entity/src/*.rs` (drop the `entities/` nesting; `mod.rs` becomes `lib.rs`)

- [ ] **Step 1: Move the entity files**

```bash
mkdir -p entity/src
git mv src-tauri/src/entities/mod.rs entity/src/lib.rs
git mv src-tauri/src/entities/camera.rs entity/src/camera.rs
git mv src-tauri/src/entities/camera_maintenance.rs entity/src/camera_maintenance.rs
git mv src-tauri/src/entities/camera_lens.rs entity/src/camera_lens.rs
git mv src-tauri/src/entities/lens.rs entity/src/lens.rs
git mv src-tauri/src/entities/lens_mount.rs entity/src/lens_mount.rs
git mv src-tauri/src/entities/film_stock.rs entity/src/film_stock.rs
git mv src-tauri/src/entities/lab.rs entity/src/lab.rs
git mv src-tauri/src/entities/roll.rs entity/src/roll.rs
git mv src-tauri/src/entities/shot.rs entity/src/shot.rs
git mv src-tauri/src/entities/shot_lens.rs entity/src/shot_lens.rs
git mv src-tauri/src/entities/development_lab.rs entity/src/development_lab.rs
git mv src-tauri/src/entities/development_self.rs entity/src/development_self.rs
git mv src-tauri/src/entities/dev_stage.rs entity/src/dev_stage.rs
git mv src-tauri/src/entities/setting.rs entity/src/setting.rs
```

- [ ] **Step 2: Create `entity/Cargo.toml`**

```toml
[package]
name = "entity"
version = "0.1.0"
edition = "2021"

[lib]
name = "entity"
path = "src/lib.rs"

[dependencies]
sea-orm = { version = "1.1", features = ["sqlx-sqlite", "runtime-tokio-rustls", "macros"], default-features = false }
serde = { version = "1.0", features = ["derive"] }
chrono = { version = "0.4", features = ["serde"] }
```

- [ ] **Step 3: Fix internal entity imports**

Inside entity files, any `crate::entities::X` now means `crate::X`. Find and rewrite:

Run: `grep -rn "crate::entities" entity/src/`
For each hit, replace `crate::entities::` with `crate::`. (`super::` references are unaffected — `super` from a module declared in `lib.rs` still resolves to the crate root.)

- [ ] **Step 4: Verify the entity crate compiles standalone**

Run: `cargo build -p entity`
Expected: PASS. (At this point the root workspace doesn't exist yet, so this step is deferred to after Task 3's workspace file — if cargo errors with "could not find workspace", proceed to Task 3 and run this verification there.)

- [ ] **Step 5: Commit**

```bash
git add entity src-tauri
git commit -m "refactor: extract entities into standalone entity crate"
```

### Task 2: Relocate the `migration` crate to repo root

**Files:**
- Move: `src-tauri/migration/` → `migration/`

The migration crate is standalone (depends only on `sea-orm-migration`, no entity/binary dependency — confirmed: all migrations `use sea_orm_migration::prelude::*` only). It moves with zero content changes.

- [ ] **Step 1: Move the crate**

```bash
git mv src-tauri/migration migration
```

- [ ] **Step 2: Confirm its Cargo.toml is unchanged and correct**

Read `migration/Cargo.toml`. Expected contents (no edits needed):
```toml
[package]
name = "migration"
version = "0.1.0"
edition = "2021"

[dependencies]
sea-orm-migration = { version = "1.1", features = ["sqlx-sqlite", "runtime-tokio-rustls"], default-features = false }
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "refactor: move migration crate to repo root"
```

### Task 3: Create the root binary crate skeleton

**Files:**
- Create: `Cargo.toml` (workspace + binary package)
- Move: `src-tauri/src/services/` → `src/services/`
- Move: `src-tauri/src/patch.rs` → `src/patch.rs`
- Create: `src/main.rs`, `src/lib.rs`, `src/config.rs`, `src/db.rs`, `src/error.rs`
- Delete: `src-tauri/` entirely (commands, lib.rs, db.rs, build.rs, tauri.conf.json, capabilities, icons, Cargo.toml)

- [ ] **Step 1: Move services and patch into the new binary crate**

```bash
mkdir -p src
git mv src-tauri/src/services src/services
git mv src-tauri/src/patch.rs src/patch.rs
```

- [ ] **Step 2: Rewrite service imports from `crate::entities::` to `entity::`**

Run: `grep -rln "crate::entities" src/services`
For each file, replace `crate::entities::` with `entity::`. Example in `src/services/camera_service.rs`:
```rust
// before
use crate::entities::camera::{self, Entity as Camera};
// after
use entity::camera::{self, Entity as Camera};
```
Do this for every service file. Re-run the grep until it returns nothing.

- [ ] **Step 3: Create the root `Cargo.toml`**

```toml
[workspace]
members = [".", "entity", "migration"]

[package]
name = "kammerz"
version = "0.1.0"
edition = "2021"
rust-version = "1.77.2"

[profile.release]
lto = "thin"
codegen-units = 1
strip = true

[dependencies]
entity = { path = "entity" }
migration = { path = "migration" }

axum = { version = "0.8", features = ["macros"] }
sea-orm = { version = "1.1", features = ["sqlx-sqlite", "runtime-tokio-rustls", "macros"], default-features = false }
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio-rustls"], default-features = false }
argon2 = { version = "0.5", features = ["std"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tower = "0.5"
tower-http = { version = "0.6", features = ["trace"] }
tower-sessions = "0.14"
tower-sessions-sqlx-store = { version = "0.15", features = ["sqlite"] }
time = "0.3"
dotenvy = "0.15"
http = "1"
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
rust-embed = "8"
mime_guess = "2"

[dev-dependencies]
tower = { version = "0.5", features = ["util"] }
http-body-util = "0.1"
```

- [ ] **Step 3b: Replace `log::` calls in services with `tracing::`**

The services currently use the `log` crate (`log::error!`, `log::warn!`, `log::info!`). The new crate uses `tracing`. They are call-compatible.

Run: `grep -rln "log::" src/services`
For each file, replace `log::error!` → `tracing::error!`, `log::warn!` → `tracing::warn!`, `log::info!` → `tracing::info!`. (No `use` line changes needed — these are fully-qualified macro calls.)

- [ ] **Step 4: Create `src/db.rs` (the critical two-phase connection)**

```rust
//! Database connection. CRITICAL: migrations must run with foreign_keys=OFF
//! because table-rebuild migrations (CREATE new → INSERT → DROP old → RENAME)
//! cascade-delete child rows under FK enforcement (see migrations 019/020 and
//! CLAUDE.md). We then enable foreign_keys=ON for runtime queries.
//!
//! We use a SINGLE-connection pool (max=min=1) so the OFF→migrate→ON pragma
//! sequence is deterministic — every query runs on the same connection, and
//! the pragma toggles can't land on a different pooled connection. A single-user
//! catalog never needs concurrent writers, and SQLite serializes writes anyway.
//! This also keeps an in-memory test DB (`sqlite::memory:`) alive for the life
//! of the pool, so integration tests can migrate + query the same database.

use std::str::FromStr;
use std::time::Duration;

use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

fn busy_timeout() -> Duration {
    let ms = std::env::var("SQLITE_BUSY_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(5000);
    Duration::from_millis(ms)
}

/// Connect (single persistent connection), migrate with FK OFF, enable FK ON.
pub async fn init(db_url: &str) -> Result<DatabaseConnection, DbErr> {
    // SqliteConnectOptions wants the path without the `sqlite:` scheme or `?query`.
    let base = db_url.strip_prefix("sqlite:").unwrap_or(db_url);
    let base = base.split('?').next().unwrap_or(base);
    let opts = SqliteConnectOptions::from_str(base)
        .map_err(|e| DbErr::Custom(format!("bad sqlite url: {e}")))?
        .create_if_missing(true)
        .busy_timeout(busy_timeout());
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .min_connections(1)
        .connect_with(opts)
        .await
        .map_err(|e| DbErr::Custom(format!("pool: {e}")))?;
    let db = sea_orm::SqlxSqliteConnector::from_sqlx_sqlite_pool(pool);

    db.execute_unprepared("PRAGMA journal_mode=WAL").await?;
    db.execute_unprepared("PRAGMA foreign_keys=OFF").await?; // critical during migrations
    Migrator::up(&db, None).await?;
    db.execute_unprepared("PRAGMA foreign_keys=ON").await?; // enforce at runtime
    Ok(db)
}

/// Default DB path. In dev: ./kammerz.db. Override with DATABASE_URL.
pub fn default_db_url() -> String {
    "sqlite:./kammerz.db?mode=rwc".to_string()
}
```

- [ ] **Step 5: Create `src/error.rs`**

```rust
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
pub enum AppError {
    Unauthorized,
    NotFound(String),
    /// User-facing message already made friendly (e.g. via friendly_err).
    UnprocessableEntity(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                "Authentication required".to_string(),
            ),
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, "NOT_FOUND", m),
            AppError::UnprocessableEntity(m) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_ERROR", m)
            }
            AppError::Internal(m) => {
                tracing::error!("internal error: {m}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An internal error occurred".to_string(),
                )
            }
        };
        (status, Json(json!({ "error": { "code": code, "message": message } }))).into_response()
    }
}

impl From<sea_orm::DbErr> for AppError {
    fn from(e: sea_orm::DbErr) -> Self {
        AppError::Internal(e.to_string())
    }
}
```

- [ ] **Step 6: Create `src/config.rs`**

```rust
#[derive(Clone, Debug)]
pub struct AppConfig {
    /// Argon2 hash of the single shared password. If None, auth is OPEN
    /// (LAN-trust mode) and a warning is logged at startup.
    pub password_hash: Option<String>,
    /// Optional Anthropic API key override. If None, the import feature falls
    /// back to the `claude_api_key` row in the settings table.
    pub anthropic_api_key: Option<String>,
    /// Session cookie Secure flag (set true when behind TLS/VPN with HTTPS).
    pub secure_cookies: bool,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            password_hash: std::env::var("KAMMERZ_PASSWORD_HASH").ok().filter(|s| !s.is_empty()),
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").ok().filter(|s| !s.is_empty()),
            secure_cookies: std::env::var("SECURE_COOKIES")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
        }
    }
}
```

- [ ] **Step 7: Create `src/lib.rs` (AppState + module wiring)**

```rust
pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod patch;
pub mod routes;
pub mod services;

use axum::extract::FromRef;
use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: config::AppConfig,
}

// Lets handlers extract `State<DatabaseConnection>` directly (chorez pattern).
impl FromRef<AppState> for DatabaseConnection {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl FromRef<AppState> for config::AppConfig {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}
```

> NOTE: `auth` and `routes` modules don't exist yet — create minimal stubs now so `lib.rs` compiles, then flesh them out in Phases 2–3.

- [ ] **Step 8: Create minimal stubs for `auth` and `routes`**

`src/auth/mod.rs`:
```rust
pub mod handlers;
pub mod middleware;
pub mod password;
```
`src/auth/password.rs`, `src/auth/handlers.rs`, `src/auth/middleware.rs`: empty for now (`// placeholder` — filled in Phase 2). Create them as empty files so the `pub mod` lines compile.

`src/routes/mod.rs`:
```rust
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::<AppState>::new()
        .route("/api/health", get(health))
        .with_state(state)
}

async fn health() -> Json<Value> {
    Json(json!({ "ok": true }))
}
```

- [ ] **Step 9: Create `src/main.rs` (bootstrap + rust-embed SPA serve)**

```rust
use axum::extract::Request;
use axum::http::{header, StatusCode, Uri};
use axum::response::IntoResponse;
use rust_embed::Embed;
use tower_http::trace::TraceLayer;

use kammerz::config::AppConfig;
use kammerz::{db, routes, AppState};

#[derive(Embed)]
#[folder = "frontend/build"]
struct Assets;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| db::default_db_url());
    let db = db::init(&db_url).await.expect("database init failed");

    let config = AppConfig::from_env();
    if config.password_hash.is_none() {
        tracing::warn!(
            "KAMMERZ_PASSWORD_HASH is not set — running in OPEN (no-auth) mode. \
             Set it for any network-reachable deployment."
        );
    }

    let state = AppState { db, config: config.clone() };

    let app = routes::create_router(state)
        .fallback(serve_spa)
        .layer(TraceLayer::new_for_http());

    let port: u16 = std::env::var("PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(3001);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("failed to bind");
    tracing::info!("kammerz listening on http://0.0.0.0:{port}");
    axum::serve(listener, app).await.expect("server error");
}

fn is_route_like(path: &str) -> bool {
    if path.is_empty() || path.starts_with("_app/") {
        return false;
    }
    let last = path.rsplit('/').next().unwrap_or(path);
    !last.contains('.')
}

async fn serve_spa(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    if path.starts_with("api/") {
        return (StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "application/json")],
            "{\"error\":{\"code\":\"NOT_FOUND\",\"message\":\"not found\"}}")
            .into_response();
    }
    let (asset, mime_path) = if path.is_empty() {
        (Assets::get("index.html"), "index.html")
    } else {
        match Assets::get(path) {
            Some(f) => (Some(f), path),
            None if is_route_like(path) => (Assets::get("index.html"), "index.html"),
            None => (None, path),
        }
    };
    match asset {
        Some(content) => {
            let mime = mime_guess::from_path(mime_path).first_or_octet_stream().as_ref().to_string();
            let cache = if path.starts_with("_app/immutable/") {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            };
            ([(header::CONTENT_TYPE, mime), (header::CACHE_CONTROL, cache.to_string())], content.data)
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
```

> NOTE: `#[folder = "frontend/build"]` must exist at compile time. Until Phase 4 moves the frontend and builds it, create a placeholder so rust-embed doesn't fail: `mkdir -p frontend/build && echo '<!doctype html><title>kammerz</title>' > frontend/build/index.html`.

- [ ] **Step 10: Remove the old Tauri crate**

```bash
git rm -r src-tauri
```

- [ ] **Step 11: Create the frontend/build placeholder so rust-embed compiles**

```bash
mkdir -p frontend/build
printf '<!doctype html><title>kammerz</title>' > frontend/build/index.html
```

- [ ] **Step 12: Build the whole workspace**

Run: `cargo build`
Expected: PASS — all three crates compile. Fix any remaining `crate::entities`/`log::` references the greps missed.

- [ ] **Step 13: Smoke-run the server**

Run: `cargo run &` then `sleep 2 && curl -s localhost:3001/api/health && curl -s localhost:3001/ | head -c 60; kill %1`
Expected: `{"ok":true}` from health, and the placeholder HTML from `/`.

- [ ] **Step 14: Commit**

```bash
git add -A
git commit -m "feat: axum binary crate skeleton (health + SPA serve), drop Tauri"
```

---

## PHASE 2 — Single-password auth

> End state: `POST /api/auth/login` verifies the shared password and starts a session; `GET /api/auth/me` reports auth status; `POST /api/auth/logout` clears it. A `RequireAuth` extractor rejects unauthenticated `/api` calls with 401. When no password hash is configured, auth is open (LAN-trust mode).

### Task 4: Password hashing CLI helper + password module

**Files:**
- Modify: `src/auth/password.rs`
- Modify: `src/main.rs` (add a `hash-password` subcommand path)
- Test: `src/auth/password.rs` (`#[cfg(test)]`)

- [ ] **Step 1: Write the failing test**

In `src/auth/password.rs`:
```rust
use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default().hash_password(password.as_bytes(), &salt)?.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else { return false; };
    Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify_roundtrips() {
        let h = hash_password("hunter2").unwrap();
        assert!(verify_password("hunter2", &h));
        assert!(!verify_password("wrong", &h));
    }
}
```

- [ ] **Step 2: Run the test to verify it passes**

Run: `cargo test -p kammerz password::tests`
Expected: PASS (this module is self-contained; the "failing" stage was the empty file not compiling).

- [ ] **Step 3: Add a `hash-password` CLI path to `main.rs`**

At the top of `main()`, before binding the server, handle a one-shot arg so the operator can generate the hash for `.env`:
```rust
let args: Vec<String> = std::env::args().collect();
if args.get(1).map(|s| s.as_str()) == Some("hash-password") {
    let pw = args.get(2).expect("usage: kammerz hash-password <password>");
    println!("{}", kammerz::auth::password::hash_password(pw).unwrap());
    return;
}
```

- [ ] **Step 4: Verify it prints a hash**

Run: `cargo run -- hash-password test123`
Expected: a string starting with `$argon2id$v=19$...`.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(auth): argon2 password hashing + hash-password CLI"
```

### Task 5: Session layer + login/logout/me handlers + RequireAuth

**Files:**
- Modify: `src/main.rs` (session store + layer)
- Modify: `src/auth/middleware.rs`, `src/auth/handlers.rs`
- Modify: `src/routes/mod.rs` (mount `/api/auth`, apply RequireAuth to business routes later)
- Test: `tests/auth.rs` (integration)

- [ ] **Step 1: Implement `src/auth/middleware.rs`**

```rust
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use tower_sessions::Session;

use crate::config::AppConfig;
use crate::error::AppError;

pub const AUTH_KEY: &str = "kammerz.authed";

pub async fn set_authed(session: &Session) -> Result<(), AppError> {
    session.insert(AUTH_KEY, true).await.map_err(|e| AppError::Internal(e.to_string()))
}

pub async fn clear_session(session: &Session) -> Result<(), AppError> {
    session.flush().await.map_err(|e| AppError::Internal(e.to_string()))
}

pub async fn is_authed(session: &Session) -> bool {
    session.get::<bool>(AUTH_KEY).await.ok().flatten().unwrap_or(false)
}

/// Extractor that rejects unauthenticated requests with 401 — UNLESS no
/// password hash is configured (open LAN-trust mode), in which case it passes.
pub struct RequireAuth;

impl<S> FromRequestParts<S> for RequireAuth
where
    S: Send + Sync,
    AppConfig: axum::extract::FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let config = AppConfig::from_ref(state);
        if config.password_hash.is_none() {
            return Ok(RequireAuth); // open mode
        }
        let session = parts.extensions.get::<Session>().ok_or(AppError::Unauthorized)?;
        if is_authed(session).await {
            Ok(RequireAuth)
        } else {
            Err(AppError::Unauthorized)
        }
    }
}
```

- [ ] **Step 2: Implement `src/auth/handlers.rs`**

```rust
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use tower_sessions::Session;

use crate::auth::middleware::{clear_session, is_authed, set_authed};
use crate::auth::password::verify_password;
use crate::config::AppConfig;
use crate::error::{AppError, AppResult};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

pub async fn login(
    State(config): State<AppConfig>,
    session: Session,
    Json(body): Json<LoginRequest>,
) -> AppResult<Json<Value>> {
    match &config.password_hash {
        // Open mode: any login succeeds (and is unnecessary).
        None => {
            set_authed(&session).await?;
            Ok(Json(json!({ "authenticated": true })))
        }
        Some(hash) => {
            if verify_password(&body.password, hash) {
                session.cycle_id().await.map_err(|e| AppError::Internal(e.to_string()))?;
                set_authed(&session).await?;
                Ok(Json(json!({ "authenticated": true })))
            } else {
                Err(AppError::Unauthorized)
            }
        }
    }
}

pub async fn logout(session: Session) -> AppResult<Json<Value>> {
    clear_session(&session).await?;
    Ok(Json(json!({ "authenticated": false })))
}

pub async fn me(State(config): State<AppConfig>, session: Session) -> Json<Value> {
    let authed = config.password_hash.is_none() || is_authed(&session).await;
    Json(json!({ "authenticated": authed, "auth_required": config.password_hash.is_some() }))
}
```

- [ ] **Step 3: Wire the session store + layer in `main.rs`**

After building `db` and before `routes::create_router`, add:
```rust
use std::str::FromStr;
use sqlx::sqlite::SqliteConnectOptions;
use tower_sessions::{cookie::SameSite, Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;
use time::Duration as TimeDuration;

let session_base = db_url.strip_prefix("sqlite:").unwrap_or(&db_url).split('?').next().unwrap().to_string();
let session_pool = sqlx::SqlitePool::connect_with(
    SqliteConnectOptions::from_str(&session_base).unwrap().create_if_missing(true).foreign_keys(true),
).await.expect("session store pool");
let session_store = SqliteStore::new(session_pool);
session_store.migrate().await.expect("session store migrate");

let session_layer = SessionManagerLayer::new(session_store)
    .with_secure(config.secure_cookies)
    .with_same_site(SameSite::Lax)
    .with_http_only(true)
    .with_expiry(Expiry::OnInactivity(TimeDuration::days(30)));
```
Then add `.layer(session_layer)` to the app builder (after `.fallback(serve_spa)`, before `TraceLayer`).

- [ ] **Step 4: Mount the auth routes in `routes/mod.rs`**

```rust
use axum::routing::{get, post};
use crate::auth::handlers;

// inside create_router, before .with_state(state):
.route("/api/auth/login", post(handlers::login))
.route("/api/auth/logout", post(handlers::logout))
.route("/api/auth/me", get(handlers::me))
```

- [ ] **Step 5: Write the failing integration test**

`tests/auth.rs`:
```rust
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

// Helper to build an app with a known password and in-memory DB.
async fn test_app(password_hash: Option<String>) -> axum::Router {
    let db = kammerz::db::init("sqlite::memory:").await.unwrap();
    let config = kammerz::config::AppConfig { password_hash, anthropic_api_key: None, secure_cookies: false };
    // Build router WITH a session layer backed by the same in-memory DB.
    let store = tower_sessions_sqlx_store::SqliteStore::new(
        sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap());
    tower_sessions::session_store::SessionStore::migrate(&store).await.unwrap();
    let layer = tower_sessions::SessionManagerLayer::new(store);
    kammerz::routes::create_router(kammerz::AppState { db, config }).layer(layer)
}

#[tokio::test]
async fn me_reports_unauthed_when_password_set() {
    let app = test_app(Some(kammerz::auth::password::hash_password("pw").unwrap())).await;
    let res = app.oneshot(Request::builder().uri("/api/auth/me").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["authenticated"], false);
    assert_eq!(v["auth_required"], true);
}

#[tokio::test]
async fn login_with_wrong_password_is_401() {
    let app = test_app(Some(kammerz::auth::password::hash_password("pw").unwrap())).await;
    let res = app.oneshot(
        Request::builder().method("POST").uri("/api/auth/login")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"password":"nope"}"#)).unwrap()
    ).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
```

- [ ] **Step 6: Run the tests**

Run: `cargo test -p kammerz --test auth`
Expected: PASS. (If `SessionStore::migrate` trait path differs in tower-sessions 0.14, adjust to `store.migrate().await` — confirm the in-scope trait.)

- [ ] **Step 7: Manual smoke test of the login cookie flow**

Run:
```bash
HASH=$(cargo run -q -- hash-password secret)
KAMMERZ_PASSWORD_HASH="$HASH" DATABASE_URL="sqlite:/tmp/kz-test.db?mode=rwc" cargo run &
sleep 3
curl -s -c /tmp/jar localhost:3001/api/auth/me
curl -s -c /tmp/jar -b /tmp/jar -XPOST localhost:3001/api/auth/login -H 'content-type: application/json' -d '{"password":"secret"}'
curl -s -b /tmp/jar localhost:3001/api/auth/me
kill %1; rm -f /tmp/kz-test.db /tmp/jar
```
Expected: first `me` → `authenticated:false`; login → `authenticated:true`; second `me` → `authenticated:true`.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat(auth): session-backed single-password login/logout/me + RequireAuth"
```

---

## PHASE 3 — API routes (one module at a time)

> End state: every former Tauri command is reachable as an `/api/*` route, guarded by `RequireAuth`. Cameras is the fully-worked exemplar; the remaining modules follow the identical handler pattern per their endpoint tables. DTOs move from the deleted command files into their route modules.

### Handler pattern (applies to every route module)

Each handler: takes `RequireAuth` first (enforces auth), then `State<DatabaseConnection>` (or `State<AppState>` when it needs config), then path/query/body extractors; calls the existing service method; maps errors with `friendly_err` for create/update/delete and a plain `AppError::Internal` for reads; returns `Json<T>` for values, `StatusCode::NO_CONTENT` for `void`, `(StatusCode::CREATED, Json(id))` for creates.

`friendly_err` moves from the old `commands/mod.rs` into `src/routes/mod.rs` (unchanged body — it's pure string logic). Re-grep to confirm no `tauri`/`State<'_, AppState>` references remain in it.

### Task 6: Cameras routes (fully-worked exemplar)

**Files:**
- Create: `src/routes/cameras.rs`
- Modify: `src/routes/mod.rs` (move `friendly_err` here; nest the cameras router)
- Test: `tests/cameras.rs`

- [ ] **Step 1: Move `friendly_err` into `src/routes/mod.rs`**

Copy the entire `friendly_err` fn from the (now-deleted) `commands/mod.rs` body shown in the inventory into `routes/mod.rs`, make it `pub fn friendly_err(...)`. Verify: `grep -n "friendly_err" src/routes/mod.rs`.

- [ ] **Step 2: Create `src/routes/cameras.rs`**

DTOs (`CreateCameraDto`, `UpdateCameraDto`, `CreateMaintenanceDto`, `UpdateMaintenanceDto`, `CreateCameraWithLensDto`) move here verbatim from the old `commands/cameras.rs` (they already derive `Deserialize` and use `double_option` — import from `crate::patch`). Handlers wrap the same service calls the old commands used.

```rust
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use sea_orm::{DatabaseConnection, DbErr, EntityTrait, Set, TransactionTrait};
use serde::Deserialize;

use entity::camera::{self, CameraFormat, CameraType};
use entity::camera_maintenance::{self, MaintenanceType};
use entity::lens;
use crate::auth::middleware::RequireAuth;
use crate::error::{AppError, AppResult};
use crate::patch::{double_option, trim, trim_opt};
use crate::routes::friendly_err;
use crate::services::camera_service::CameraService;
use crate::services::lens_service::LensService;
use crate::AppState;

// --- DTOs (moved verbatim from commands/cameras.rs) ---
// CreateCameraDto, UpdateCameraDto, CreateMaintenanceDto, UpdateMaintenanceDto,
// CreateCameraWithLensDto — paste field-for-field from the inventory.

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/with-lens", post(create_with_lens))
        .route("/distinct/brands", get(distinct_brands))
        .route("/distinct/vendors", get(distinct_vendors))
        .route("/distinct/maint-providers", get(distinct_maint_providers))
        .route("/{id}", get(get_one).put(update).delete(delete_one))
        .route("/{id}/lenses", get(lenses_for_camera))
        .route("/{id}/lenses/{lens_id}", post(link_lens).delete(unlink_lens))
        .route("/{id}/maintenance", get(list_maintenance))
}

async fn list(_: RequireAuth, State(db): State<DatabaseConnection>)
    -> AppResult<Json<Vec<camera::Model>>>
{
    CameraService::list_all(&db).await.map(Json).map_err(|e| AppError::Internal(e.to_string()))
}

async fn get_one(_: RequireAuth, State(db): State<DatabaseConnection>, Path(id): Path<i32>)
    -> AppResult<Json<Option<camera::Model>>>
{
    CameraService::get_by_id(&db, id).await.map(Json).map_err(|e| AppError::Internal(e.to_string()))
}

async fn create(_: RequireAuth, State(db): State<DatabaseConnection>, Json(data): Json<CreateCameraDto>)
    -> AppResult<(StatusCode, Json<i32>)>
{
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let model = camera::ActiveModel {
        brand: trim(data.brand), model: trim(data.model), prefix: trim_opt(data.prefix),
        format: Set(data.format), lens_mount_id: Set(data.lens_mount_id),
        default_lens_id: Set(data.default_lens_id), camera_type: Set(data.camera_type),
        serial_number: trim_opt(data.serial_number), date_purchased: trim_opt(data.date_purchased),
        purchased_from: trim_opt(data.purchased_from), date_sold: trim_opt(data.date_sold),
        notes: trim_opt(data.notes), created_at: Set(now.clone()), updated_at: Set(now),
        ..Default::default()
    };
    let res = CameraService::create(&db, model).await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("camera", e)))?;
    Ok((StatusCode::CREATED, Json(res.id)))
}

async fn update(_: RequireAuth, State(db): State<DatabaseConnection>, Path(id): Path<i32>, Json(data): Json<UpdateCameraDto>)
    -> AppResult<StatusCode>
{
    let existing = CameraService::get_by_id(&db, id).await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Camera {id} not found")))?;
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut model: camera::ActiveModel = existing.into();
    if let Some(v) = data.brand { model.brand = trim(v); }
    if let Some(v) = data.model { model.model = trim(v); }
    if let Some(v) = data.prefix { model.prefix = trim_opt(v); }
    if let Some(v) = data.format { model.format = Set(v); }
    if let Some(v) = data.lens_mount_id { model.lens_mount_id = Set(v); }
    if let Some(v) = data.default_lens_id { model.default_lens_id = Set(v); }
    if let Some(v) = data.camera_type { model.camera_type = Set(v); }
    if let Some(v) = data.serial_number { model.serial_number = trim_opt(v); }
    if let Some(v) = data.date_purchased { model.date_purchased = trim_opt(v); }
    if let Some(v) = data.purchased_from { model.purchased_from = trim_opt(v); }
    if let Some(v) = data.date_sold { model.date_sold = trim_opt(v); }
    if let Some(v) = data.notes { model.notes = trim_opt(v); }
    model.updated_at = Set(now);
    CameraService::update(&db, model).await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("camera", e)))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_one(_: RequireAuth, State(db): State<DatabaseConnection>, Path(id): Path<i32>)
    -> AppResult<StatusCode>
{
    CameraService::delete(&db, id).await
        .map_err(|e| AppError::UnprocessableEntity(friendly_err("camera", e)))?;
    Ok(StatusCode::NO_CONTENT)
}

// distinct_brands/vendors/maint_providers: each `_: RequireAuth, State(db)` →
// CameraService::distinct_*  → Json<Vec<String>>, map_err Internal.
// lenses_for_camera(Path(id)) → CameraService::get_lenses_for_camera → Json<Vec<i32>>.
// link_lens(Path((id, lens_id))) → CameraService::link_lens → 204.
// unlink_lens(Path((id, lens_id))) → CameraService::unlink_lens → 204.
// list_maintenance(Path(id)) → CameraService::list_maintenance → Json<Vec<..>>.
// create_with_lens(Json<CreateCameraWithLensDto>) → paste the transactional body
//   from commands/cameras.rs::create_camera_with_lens verbatim (State<DatabaseConnection>
//   replaces State<AppState>; `state.db` becomes `db`).
```

> Write out the handlers sketched in the trailing comment in full, following the exact bodies from the inventory's `commands/cameras.rs` — the only mechanical changes are: drop `#[tauri::command]`, add `_: RequireAuth` as first param, `State<'_, AppState>`→`State<DatabaseConnection>`, `state.db`→`db`, error strings→`AppError`, and return `Json`/`StatusCode` instead of `Result<T, String>`.

- [ ] **Step 3: Create the maintenance router (`POST/PUT/DELETE /api/maintenance`)**

Put `create_maintenance`, `update_maintenance`, `delete_maintenance` in `src/routes/cameras.rs` too, exposed via a second `maintenance_router()` returning `Router<AppState>` with routes `/` (POST) and `/{id}` (PUT, DELETE).

- [ ] **Step 4: Nest cameras + maintenance in `routes/mod.rs`**

```rust
// inside create_router:
.nest("/api/cameras", cameras::router())
.nest("/api/maintenance", cameras::maintenance_router())
```

- [ ] **Step 5: Write the failing integration test**

`tests/cameras.rs` — reuse the `test_app` helper pattern from `tests/auth.rs` (factor it into `tests/common/mod.rs`). With open-auth mode (`password_hash: None`) to skip session setup:
```rust
#[tokio::test]
async fn list_cameras_returns_seeded_gear() {
    // Fresh in-memory DB runs all migrations incl. seed_user_cameras.
    let app = open_app().await;
    let res = app.oneshot(get("/api/cameras")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let cams: Vec<serde_json::Value> = json_body(res).await;
    assert!(!cams.is_empty(), "migrations seed the user's cameras");
}

#[tokio::test]
async fn create_then_get_camera_roundtrips() { /* POST /api/cameras → 201 id; GET /api/cameras/{id} → that camera */ }
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p kammerz --test cameras`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat(api): cameras + maintenance routes (exemplar)"
```

### Task 7: Remaining CRUD route modules

For each module below, create `src/routes/<module>.rs` following the Task 6 exemplar exactly, move its DTOs from the old command file, add `.nest("/api/<base>", <module>::router())` in `routes/mod.rs`, and write one smoke test (`list`/`get` returns 200; one create roundtrip where applicable). These handlers are thin pass-throughs to already-working services — the service behavior was preserved verbatim in Phase 1.

**Endpoint tables (method · path · former command · service call · body/return):**

- [ ] **Step 1: `lenses.rs`** — nest at `/api/lenses`
  - GET `/` · list_lenses · `LensService::list_all` · → `Vec<lens::Model>`
  - POST `/` · create_lens · `LensService::create` · body `CreateLensDto` → 201 `i32`
  - GET `/distinct/brands` · list_distinct_lens_brands · `LensService::distinct_brands` · → `Vec<String>`
  - GET `/distinct/systems` · list_distinct_lens_systems · `LensService::distinct_lens_systems` · → `Vec<String>`
  - GET `/{id}` · get_lens · `LensService::get_by_id` · → `Option<lens::Model>`
  - PUT `/{id}` · update_lens · `LensService::update` · body `UpdateLensDto` → 204
  - DELETE `/{id}` · delete_lens · `LensService::delete` · → 204
  - GET `/{id}/cameras` · get_cameras_for_lens · `LensService::get_cameras_for_lens` · → `Vec<i32>`

- [ ] **Step 2: `lens_mounts.rs`** — nest at `/api/lens-mounts`
  - GET `/` · list_lens_mounts · `LensMountService::list_all` · → `Vec<lens_mount::Model>`
  - POST `/` · create_lens_mount · `LensMountService::create` · body `{ "name": String }` → 201 `i32`

- [ ] **Step 3: `film_stocks.rs`** — nest at `/api/film-stocks`
  - GET `/` · list_film_stocks · `FilmStockService::list_all`
  - POST `/` · create_film_stock · `FilmStockService::create` · body `CreateFilmStockDto` → 201 `i32`
  - GET `/distinct/brands` · list_distinct_film_brands · `FilmStockService::distinct_brands`
  - GET `/{id}` · get_film_stock · PUT `/{id}` · update_film_stock (`UpdateFilmStockDto`) → 204 · DELETE `/{id}` · delete_film_stock → 204

- [ ] **Step 4: `labs.rs`** — nest at `/api/labs`
  - GET `/` list · POST `/` create (`CreateLabDto`) → 201 · GET `/{id}` · PUT `/{id}` (`UpdateLabDto`) → 204 · DELETE `/{id}` → 204

- [ ] **Step 5: `rolls.rs`** — nest at `/api/rolls` (move `RollDetail` DTO here; `RollWithDetails` stays in `roll_service`)
  - GET `/` · list_rolls · `RollService::list_all_with_details` · → `Vec<RollWithDetails>`
  - POST `/` · create_roll · `RollService::create` · body `CreateRollDto` → 201 `i32`
  - GET `/suggest-id` · suggest_roll_id · `RollService::suggest_id` · → `String`
  - GET `/for-camera/{camera_id}` · list_rolls_for_camera · `RollService::list_for_camera` · → `Vec<RollWithDetails>`
  - GET `/{id}` · get_roll · `RollService::get_with_details` · → `Option<RollWithDetails>`
  - GET `/{id}/detail` · get_roll_detail · (composite — paste the aggregation body from `commands/rolls.rs::get_roll_detail`) · → `RollDetail`
  - PUT `/{id}` · update_roll (`UpdateRollDto`) → 204 · DELETE `/{id}` · delete_roll → 204

- [ ] **Step 6: `shots.rs`** — nest at `/api/shots` (transactional creates/deletes — paste txn bodies verbatim)
  - GET `/for-roll/{roll_id}` · list_shots_for_roll · `ShotService::list_for_roll`
  - GET `/for-roll/{roll_id}/lenses` · get_lenses_for_roll_shots · → `Vec<(i32,i32)>`
  - GET `/for-roll/{roll_id}/count` · count_shots_for_roll · → `u64`
  - GET `/for-roll/{roll_id}/next-frame` · suggest_next_frame · → `String`
  - POST `/` · create_shot · (txn: `ShotService::create` + `set_lenses_for_shot` + `RollService::auto_sync_status`) · body `CreateShotDto` → 201 `i32`
  - GET `/{id}` · get_shot · PUT `/{id}` · update_shot (txn) → 204 · DELETE `/{id}` · delete_shot (txn) → 204
  - GET `/{id}/lenses` · get_lenses_for_shot · → `Vec<i32>`

- [ ] **Step 7: `development.rs`** — nest at `/api/development` (move `CreateLabDevDto`, `UpdateLabDevDto`, `CreateSelfDevDto`, `UpdateSelfDevDto`, `StageDto`; `SelfDevWithStages` stays in `development_service`)
  - GET `/lab/for-roll/{roll_id}` · get_lab_dev_for_roll
  - POST `/lab` · create_lab_dev (txn + auto_sync_status) → 201 · PUT `/lab/{id}` · update_lab_dev → 204 · DELETE `/lab/{id}` · delete_lab_dev (txn) → 204
  - GET `/self/for-roll/{roll_id}` · get_self_dev_for_roll
  - GET `/self` · list_all_self_developments · → `Vec<SelfDevWithStages>`
  - POST `/self` · create_self_dev (txn + set_stages + auto_sync) → 201 · PUT `/self/{id}` · update_self_dev (txn + set_stages) → 204 · DELETE `/self/{id}` · delete_self_dev (txn) → 204
  - GET `/self/{id}/stages` · list_dev_stages · → `Vec<dev_stage::Model>`

- [ ] **Step 8: `search.rs`** — nest at `/api/search`
  - GET `/?q=<query>` · search_catalog · `SearchService::search` · extract `Query<{ q: String }>` → `SearchResults`

- [ ] **Step 9: `stats.rs`** — nest at `/api/stats`
  - GET `/` · get_catalog_stats · `StatsService::get_stats` · → `CatalogStats`

- [ ] **Step 10: `settings.rs`** — nest at `/api/settings`
  - GET `/{key}` · get_setting · `SettingsService::get_setting` · → `Option<String>`
  - PUT `/{key}` · set_setting · `SettingsService::set_setting` · body `{ "value": String }` → 204

- [ ] **Step 11: Build + run all module smoke tests**

Run: `cargo test -p kammerz`
Expected: PASS for every module test.

- [ ] **Step 12: Commit**

```bash
git add -A
git commit -m "feat(api): port all CRUD route modules from Tauri commands"
```

### Task 8: Import routes (Anthropic via reqwest, server-side key)

**Files:**
- Create: `src/routes/import.rs`
- Modify: `src/routes/mod.rs`
- Note: `src/services/import_service.rs` already moved in Phase 1 (uses `reqwest`).

The old `import.rs` command read `claude_api_key`/`claude_model` from the settings table. Preserve that, but let `AppConfig.anthropic_api_key` (env `ANTHROPIC_API_KEY`) override it — this satisfies the spec's "API key in server config" intent while keeping the existing settings-UI flow working.

- [ ] **Step 1: Create `src/routes/import.rs`**

```rust
// helper: resolve key = config.anthropic_api_key OR settings 'claude_api_key'
async fn resolve_key(db: &DatabaseConnection, config: &AppConfig) -> AppResult<String> {
    if let Some(k) = &config.anthropic_api_key { return Ok(k.clone()); }
    SettingsService::get_setting(db, "claude_api_key").await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::UnprocessableEntity(
            "No Anthropic API key configured. Set it in Settings or the ANTHROPIC_API_KEY env var.".into()))
}
```
Routes (nest at `/api/import`):
- GET `/models` · list_models · `ImportService::list_models(&key)` · → `Vec<ModelInfo>`
- POST `/parse` · parse_note · body `{ note_text: String, model: Option<String> }`; resolve model = body.model OR settings `claude_model` OR `DEFAULT_MODEL` (keep the `DEFAULT_MODEL` const from the old command file) · `ImportService::parse_note(&key, &model, &note_text)` · → `ParsedRoll`
- POST `/roll` · import_parsed_roll · body `ImportRollDto` (move it + `ImportShotDto` here) · `RollService::import_roll` (txn) → 201 `i32`

Map `ImportService` errors (already `String`) to `AppError::UnprocessableEntity`.

- [ ] **Step 2: Smoke test (no network)**

`tests/import.rs`: with no key configured, `GET /api/import/models` returns 422 with the "No Anthropic API key" message. (Don't hit the real API in tests.)

Run: `cargo test -p kammerz --test import`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(api): import routes with server-side Anthropic key resolution"
```

### Task 9: Apply `RequireAuth` consistently + a guard regression test

**Files:**
- Test: `tests/auth_guard.rs`

Every business handler already takes `_: RequireAuth`. Add a test proving the guard actually blocks.

- [ ] **Step 1: Write the failing test**

```rust
#[tokio::test]
async fn business_routes_require_auth_when_password_set() {
    let app = app_with_password("pw").await; // password_hash: Some(...)
    let res = app.oneshot(get("/api/cameras")).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn business_routes_open_when_no_password() {
    let app = open_app().await; // password_hash: None
    let res = app.oneshot(get("/api/cameras")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
```

- [ ] **Step 2: Run + commit**

Run: `cargo test -p kammerz --test auth_guard` → PASS.
```bash
git add -A && git commit -m "test(api): assert RequireAuth gates business routes"
```

---

## PHASE 4 — Frontend: move under `frontend/`, swap invoke→fetch, add auth UI

> End state: SvelteKit lives in `frontend/`, builds to `frontend/build`, talks to axum via a shared `fetch` helper, gates all pages behind a `/login` screen, and contains zero `@tauri-apps` references.

### Task 10: Move the SvelteKit app into `frontend/`

**Files:** moves only

- [ ] **Step 1: Move frontend files**

```bash
mkdir -p frontend
git mv src frontend/src
git mv static frontend/static
git mv package.json frontend/package.json
git mv svelte.config.js frontend/svelte.config.js
git mv vite.config.ts frontend/vite.config.ts
git mv tsconfig.json frontend/tsconfig.json
test -e .npmrc && git mv .npmrc frontend/.npmrc || true
test -e bun.lock && git mv bun.lock frontend/bun.lock || true
test -e bun.lockb && git mv bun.lockb frontend/bun.lockb || true
```

> The `frontend/build/` placeholder from Phase 1 stays; real builds overwrite it.

- [ ] **Step 2: Update `frontend/vite.config.ts` — drop Tauri dev config, add API proxy**

```typescript
import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

const backendPort = Number(process.env.PORT) || 3001;

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	server: {
		port: 5173,
		proxy: {
			'/api': { target: `http://localhost:${backendPort}`, changeOrigin: true }
		}
	}
});
```

- [ ] **Step 3: Update `frontend/package.json` — remove Tauri deps + script**

Remove `@tauri-apps/api`, `@tauri-apps/cli` from devDependencies and the `"tauri": "tauri"` script. Keep everything else.

- [ ] **Step 4: Verify the frontend still builds in isolation**

Run: `cd frontend && bun install && bun run build && cd ..`
Expected: build fails ONLY on the `invoke` imports we're about to replace (TypeScript may still build since `@tauri-apps/api` is now uninstalled — note the errors; they're fixed in Task 11). If it builds clean because TS isn't strict on the missing module, that's fine too.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor(frontend): relocate SvelteKit app under frontend/"
```

### Task 11: Shared fetch helper + rewrite the 13 api wrappers

**Files:**
- Create: `frontend/src/lib/api/client.ts`
- Modify: all `frontend/src/lib/api/*.ts`

- [ ] **Step 1: Create the shared client (`frontend/src/lib/api/client.ts`)**

```typescript
export interface ApiErrorShape { code: string; message: string; status: number; }

export class ApiRequestError extends Error {
	code: string;
	status: number;
	constructor(e: ApiErrorShape) {
		super(e.message);
		this.name = 'ApiRequestError';
		this.code = e.code;
		this.status = e.status;
	}
}

let onUnauthorized: (() => void) | null = null;
export function setUnauthorizedHandler(fn: (() => void) | null) { onUnauthorized = fn; }

export async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
	const init: RequestInit = { method, credentials: 'include', headers: {} };
	if (body !== undefined) {
		(init.headers as Record<string, string>)['Content-Type'] = 'application/json';
		init.body = JSON.stringify(body);
	}
	let res: Response;
	try {
		res = await fetch(path, init);
	} catch (err) {
		throw new ApiRequestError({ code: 'NETWORK', message: err instanceof Error ? err.message : 'Network error', status: 0 });
	}
	if (res.status === 401 && onUnauthorized) onUnauthorized();
	if (!res.ok) {
		let shape: ApiErrorShape;
		try {
			const j = await res.json();
			shape = { code: j.error?.code ?? 'UNKNOWN', message: j.error?.message ?? res.statusText, status: res.status };
		} catch {
			shape = { code: 'UNKNOWN', message: res.statusText, status: res.status };
		}
		throw new ApiRequestError(shape);
	}
	if (res.status === 204) return undefined as T;
	return res.json() as Promise<T>;
}

// Query-string helper for endpoints that take params.
export function qs(params: Record<string, string | number | undefined>): string {
	const p = new URLSearchParams();
	for (const [k, v] of Object.entries(params)) if (v !== undefined) p.set(k, String(v));
	const s = p.toString();
	return s ? `?${s}` : '';
}
```

- [ ] **Step 2: Rewrite `cameras.ts` (exemplar)**

```typescript
import type { Camera, CameraInsert, CameraMaintenance, CameraMaintenanceInsert } from '$lib/types';
import { request } from './client';

export const listCameras = () => request<Camera[]>('GET', '/api/cameras');
export const getCamera = (id: number) => request<Camera | null>('GET', `/api/cameras/${id}`);
export const createCamera = (data: CameraInsert) => request<number>('POST', '/api/cameras', data);
export const updateCamera = (id: number, data: Partial<CameraInsert>) =>
	request<void>('PUT', `/api/cameras/${id}`, data);
export const deleteCamera = (id: number) => request<void>('DELETE', `/api/cameras/${id}`);

export const listMaintenanceForCamera = (cameraId: number) =>
	request<CameraMaintenance[]>('GET', `/api/cameras/${cameraId}/maintenance`);
export const createMaintenance = (data: CameraMaintenanceInsert) =>
	request<number>('POST', '/api/maintenance', data);
export const updateMaintenance = (id: number, data: Partial<CameraMaintenanceInsert>) =>
	request<void>('PUT', `/api/maintenance/${id}`, data);
export const deleteMaintenance = (id: number) => request<void>('DELETE', `/api/maintenance/${id}`);

export interface CreateCameraWithLensData {
	camera: CameraInsert;
	lens_model: string | null;
	lens_focal_length: string | null;
	lens_max_aperture: string | null;
}
export const createCameraWithLens = (data: CreateCameraWithLensData) =>
	request<number>('POST', '/api/cameras/with-lens', data);

export const getLensesForCamera = (cameraId: number) =>
	request<number[]>('GET', `/api/cameras/${cameraId}/lenses`);
export const linkLensToCamera = (cameraId: number, lensId: number) =>
	request<void>('POST', `/api/cameras/${cameraId}/lenses/${lensId}`);
export const unlinkLensFromCamera = (cameraId: number, lensId: number) =>
	request<void>('DELETE', `/api/cameras/${cameraId}/lenses/${lensId}`);

export const listDistinctCameraBrands = () => request<string[]>('GET', '/api/cameras/distinct/brands');
export const listDistinctVendors = () => request<string[]>('GET', '/api/cameras/distinct/vendors');
export const listDistinctMaintProviders = () =>
	request<string[]>('GET', '/api/cameras/distinct/maint-providers');
```

- [ ] **Step 3: Rewrite the remaining wrappers**

Apply the identical mechanical change (`invoke<T>('cmd', args)` → `request<T>(METHOD, PATH, body?)`) per the path tables from Phase 3. Exact mappings:

**lenses.ts:** `listLenses`→GET `/api/lenses` · `getLens(id)`→GET `/api/lenses/${id}` · `createLens(data)`→POST `/api/lenses` · `updateLens(id,data)`→PUT `/api/lenses/${id}` · `deleteLens(id)`→DELETE `/api/lenses/${id}` · `listDistinctLensBrands`→GET `/api/lenses/distinct/brands` · `getCamerasForLens(lensId)`→GET `/api/lenses/${lensId}/cameras`

**lens-mounts.ts:** `listLensMounts`→GET `/api/lens-mounts` · `createLensMount(name)`→POST `/api/lens-mounts`, body `{ name }`

**film-stocks.ts:** `listFilmStocks`→GET `/api/film-stocks` · `getFilmStock(id)`→GET `/api/film-stocks/${id}` · `createFilmStock(data)`→POST `/api/film-stocks` · `updateFilmStock(id,data)`→PUT `/api/film-stocks/${id}` · `deleteFilmStock(id)`→DELETE `/api/film-stocks/${id}` · `listDistinctFilmBrands`→GET `/api/film-stocks/distinct/brands`

**labs.ts:** `listLabs`→GET `/api/labs` · `getLab(id)`→GET `/api/labs/${id}` · `createLab(data)`→POST `/api/labs` · `updateLab(id,data)`→PUT `/api/labs/${id}` · `deleteLab(id)`→DELETE `/api/labs/${id}`

**rolls.ts:** `listRolls`→GET `/api/rolls` · `getRoll(id)`→GET `/api/rolls/${id}` · `getRollDetail(id)`→GET `/api/rolls/${id}/detail` · `createRoll(data)`→POST `/api/rolls` · `updateRoll(id,data)`→PUT `/api/rolls/${id}` · `deleteRoll(id)`→DELETE `/api/rolls/${id}` · `listRollsForCamera(cameraId)`→GET `/api/rolls/for-camera/${cameraId}` · `suggestRollId`→GET `/api/rolls/suggest-id`

**shots.ts:** `listShotsForRoll(rollId)`→GET `/api/shots/for-roll/${rollId}` · `getShot(id)`→GET `/api/shots/${id}` · `createShot(data)`→POST `/api/shots` · `updateShot(id,data)`→PUT `/api/shots/${id}` · `deleteShot(id)`→DELETE `/api/shots/${id}` · `getLensesForShot(shotId)`→GET `/api/shots/${shotId}/lenses` · `getLensesForRollShots(rollId)`→GET `/api/shots/for-roll/${rollId}/lenses` · `suggestNextFrame(rollId)`→GET `/api/shots/for-roll/${rollId}/next-frame` · `countShotsForRoll(rollId)`→GET `/api/shots/for-roll/${rollId}/count`

**development.ts:** `getLabDevForRoll(rollId)`→GET `/api/development/lab/for-roll/${rollId}` · `createLabDev(data)`→POST `/api/development/lab` · `updateLabDev(id,data)`→PUT `/api/development/lab/${id}` · `deleteLabDev(id)`→DELETE `/api/development/lab/${id}` · `getSelfDevForRoll(rollId)`→GET `/api/development/self/for-roll/${rollId}` · `createSelfDev(data)`→POST `/api/development/self` · `updateSelfDev(id,data)`→PUT `/api/development/self/${id}` · `deleteSelfDev(id)`→DELETE `/api/development/self/${id}` · `listDevStages(developmentSelfId)`→GET `/api/development/self/${developmentSelfId}/stages` · `listAllSelfDevelopments`→GET `/api/development/self`

**search.ts:** `searchCatalog(query)`→GET `/api/search${qs({ q: query })}` (import `qs` from `./client`)

**stats.ts:** `getCatalogStats`→GET `/api/stats`

**settings.ts:** `getSetting(key)`→GET `/api/settings/${encodeURIComponent(key)}` · `setSetting(key,value)`→PUT `/api/settings/${encodeURIComponent(key)}`, body `{ value }`

**import.ts:** `listModels`→GET `/api/import/models` · `parseNote(noteText, model?)`→POST `/api/import/parse`, body `{ note_text: noteText, model }` · `importParsedRoll(data)`→POST `/api/import/roll`

> NOTE the `parseNote` body key rename: the backend DTO field is `note_text` (snake_case), so send `{ note_text: noteText, model }`, not `{ noteText }`.

- [ ] **Step 4: Confirm zero Tauri references remain**

Run: `grep -rn "@tauri-apps\|invoke(" frontend/src && echo "FOUND — fix above" || echo "clean"`
Expected: `clean`.

- [ ] **Step 5: Type-check**

Run: `cd frontend && bun run check && cd ..`
Expected: PASS (no missing-module errors, types unchanged since return types are identical).

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor(frontend): replace Tauri invoke with shared fetch client"
```

### Task 12: Auth UI — store, login page, route guard

**Files:**
- Create: `frontend/src/lib/api/auth.ts`
- Create: `frontend/src/lib/stores/auth.svelte.ts`
- Create: `frontend/src/routes/login/+page.svelte`
- Create: `frontend/src/routes/(app)/+layout.svelte` (the Sidebar shell, moved)
- Create: `frontend/src/routes/(app)/+layout.ts` (auth guard)
- Modify: `frontend/src/routes/+layout.svelte` (strip Sidebar; keep app.css + ssr/prerender)
- Move: all existing route dirs into `frontend/src/routes/(app)/`

- [ ] **Step 1: Auth API + store**

`frontend/src/lib/api/auth.ts`:
```typescript
import { request } from './client';
export interface AuthStatus { authenticated: boolean; auth_required: boolean; }
export const getAuthStatus = () => request<AuthStatus>('GET', '/api/auth/me');
export const login = (password: string) =>
	request<{ authenticated: boolean }>('POST', '/api/auth/login', { password });
export const logout = () => request<{ authenticated: boolean }>('POST', '/api/auth/logout');
```

`frontend/src/lib/stores/auth.svelte.ts`:
```typescript
import { getAuthStatus, login as apiLogin, logout as apiLogout } from '$lib/api/auth';
import { setUnauthorizedHandler } from '$lib/api/client';

let authenticated = $state(false);
let authRequired = $state(true);
let initialized = $state(false);

setUnauthorizedHandler(() => { authenticated = false; });

export const auth = {
	get authenticated() { return authenticated; },
	get authRequired() { return authRequired; },
	get initialized() { return initialized; },
	async init() {
		const s = await getAuthStatus();
		authenticated = s.authenticated;
		authRequired = s.auth_required;
		initialized = true;
	},
	async login(password: string) {
		const r = await apiLogin(password);
		authenticated = r.authenticated;
		return r.authenticated;
	},
	async logout() { await apiLogout(); authenticated = false; }
};
```

- [ ] **Step 2: Move existing routes under `(app)/`**

```bash
cd frontend/src/routes
mkdir -p '(app)'
for d in cameras lenses lens-mounts film-stocks labs rolls developments shots search stats settings import quick-entry; do
  test -e "$d" && git mv "$d" "(app)/$d" || true
done
test -e +page.svelte && git mv +page.svelte '(app)/+page.svelte' || true
cd ../../..
```
(Adjust the dir list to match the actual `routes/` listing — include every page dir except `login`.)

- [ ] **Step 3: Move the Sidebar shell into `(app)/+layout.svelte`**

Create `frontend/src/routes/(app)/+layout.svelte` with the current Sidebar markup:
```svelte
<script lang="ts">
	import Sidebar from '$lib/components/layout/Sidebar.svelte';
	let { children } = $props();
</script>

<div class="flex h-screen overflow-hidden">
	<Sidebar />
	<main class="flex flex-1 flex-col overflow-y-auto">
		{@render children()}
	</main>
</div>
```

- [ ] **Step 4: Slim the root `+layout.svelte`**

```svelte
<script lang="ts">
	import '../app.css';
	let { children } = $props();
</script>

<svelte:head><title>Kammerz</title></svelte:head>
{@render children()}
```
Keep `frontend/src/routes/+layout.ts` as-is (`export const ssr = false; export const prerender = false;`).

- [ ] **Step 5: Auth guard in `(app)/+layout.ts`**

```typescript
import { auth } from '$lib/stores/auth.svelte';
import { redirect } from '@sveltejs/kit';
import type { LayoutLoad } from './$types';

export const load: LayoutLoad = async ({ url }) => {
	if (!auth.initialized) await auth.init();
	if (auth.authRequired && !auth.authenticated) {
		const next = url.pathname + url.search;
		throw redirect(307, `/login?next=${encodeURIComponent(next)}`);
	}
	return {};
};
```

- [ ] **Step 6: Login page `frontend/src/routes/login/+page.svelte`**

```svelte
<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { auth } from '$lib/stores/auth.svelte';
	import { ApiRequestError } from '$lib/api/client';

	let password = $state('');
	let error = $state('');
	let submitting = $state(false);

	function safeNext(): string {
		const raw = page.url.searchParams.get('next');
		if (!raw || !raw.startsWith('/')) return '/';
		return raw;
	}

	async function submit() {
		error = '';
		submitting = true;
		try {
			const ok = await auth.login(password);
			if (ok) goto(safeNext());
			else error = 'Incorrect password';
		} catch (e) {
			error = e instanceof ApiRequestError && e.status === 401 ? 'Incorrect password' : 'Something went wrong';
		} finally {
			submitting = false;
		}
	}
</script>

<svelte:head><title>Sign in — Kammerz</title></svelte:head>

<div class="flex min-h-screen items-center justify-center bg-surface">
	<div class="w-full max-w-sm rounded-lg border border-border-subtle bg-surface-raised p-8">
		<h1 class="mb-6 font-display text-3xl text-accent">Kammerz</h1>
		{#if error}<p class="mb-4 text-sm text-status-developing">{error}</p>{/if}
		<input
			type="password"
			bind:value={password}
			placeholder="Password"
			autocomplete="current-password"
			onkeydown={(e) => e.key === 'Enter' && submit()}
			class="mb-4 h-[38px] w-full rounded border border-border bg-surface px-3 text-text"
		/>
		<button
			onclick={submit}
			disabled={submitting}
			class="h-[38px] w-full rounded bg-accent font-semibold text-surface disabled:opacity-60"
		>
			{submitting ? 'Signing in…' : 'Sign in'}
		</button>
	</div>
</div>
```
(Use the actual `design-system` tokens/classes — this is a first pass; the `design-system` skill applies on the styling polish.)

- [ ] **Step 7: Build**

Run: `cd frontend && bun run build && cd ..`
Expected: PASS — outputs `frontend/build/` with `index.html` + `_app/`.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat(frontend): login page, auth store, (app) route group guard"
```

---

## PHASE 5 — Dev tooling, embedded build, deployment, docs

### Task 13: justfile + end-to-end release build

**Files:**
- Create/Modify: `justfile`

- [ ] **Step 1: Write the `justfile`**

```just
# Run backend (axum, :3001) and frontend (vite, :5173) together for dev.
dev:
    #!/usr/bin/env bash
    trap 'kill 0' EXIT
    cargo run &
    (cd frontend && bun run dev) &
    wait

dev-backend:
    cargo run

dev-frontend:
    cd frontend && bun run dev

# Production build: frontend → frontend/build, then embed into the release binary.
build:
    cd frontend && bun install && bun run build
    cargo build --release

check:
    cd frontend && bun run check
    cargo build
    cargo test

migrate:
    cargo run -- # migrations run on startup; this just boots once
    
# Generate the argon2 hash for KAMMERZ_PASSWORD_HASH.
hash-password password:
    cargo run -q -- hash-password {{password}}
```

- [ ] **Step 2: Full release build (proves rust-embed picks up the real frontend)**

Run: `just build`
Expected: `frontend/build/` regenerated, then `cargo build --release` embeds it. Binary at `target/release/kammerz`.

- [ ] **Step 3: Run the release binary and verify it serves the real app + API**

Run:
```bash
HASH=$(target/release/kammerz hash-password secret)
KAMMERZ_PASSWORD_HASH="$HASH" DATABASE_URL="sqlite:/tmp/kz.db?mode=rwc" PORT=3001 target/release/kammerz &
sleep 3
curl -s localhost:3001/api/auth/me
curl -s -o /dev/null -w "%{http_code}\n" localhost:3001/        # 200, real index.html
curl -s -o /dev/null -w "%{http_code}\n" localhost:3001/cameras # 200, SPA fallback
curl -s -o /dev/null -w "%{http_code}\n" localhost:3001/api/cameras # 401 (auth required)
kill %1; rm -f /tmp/kz.db
```
Expected: `me` → auth_required:true; `/` and `/cameras` → 200; `/api/cameras` → 401.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "chore: justfile dev/build recipes + verify embedded release build"
```

### Task 14: Deployment artifacts (.env.example + systemd unit)

**Files:**
- Create: `.env.example`
- Create: `deploy/kammerz.service`

- [ ] **Step 1: `.env.example`**

```bash
# Where the SQLite catalog lives on the NAS.
DATABASE_URL=sqlite:/opt/kammerz/data/kammerz.db?mode=rwc
# Argon2 hash of the single shared password (generate: kammerz hash-password <pw>).
KAMMERZ_PASSWORD_HASH=
# Set true only when served over HTTPS (behind a TLS reverse proxy).
SECURE_COOKIES=false
# Listen port.
PORT=3001
# Optional: overrides the claude_api_key settings row for the AI import feature.
ANTHROPIC_API_KEY=
```

- [ ] **Step 2: `deploy/kammerz.service`** (mirrors chorez's hardened unit)

```ini
[Unit]
Description=Kammerz film catalog
After=network.target

[Service]
Type=simple
User=kammerz
WorkingDirectory=/opt/kammerz
EnvironmentFile=/opt/kammerz/.env
ExecStart=/opt/kammerz/kammerz
Restart=on-failure
StartLimitIntervalSec=300
StartLimitBurst=5
ProtectSystem=strict
ReadWritePaths=/opt/kammerz/data
ProtectHome=true
PrivateTmp=true
NoNewPrivileges=true

[Install]
WantedBy=multi-user.target
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "chore(deploy): .env.example + hardened systemd unit"
```

### Task 15: Data migration note + DB bootstrap verification

**Files:** none (operational — documented in README in Task 16)

- [ ] **Step 1: Decide the data path and verify both bootstrap routes**

Two cases:
- **Carry over existing catalog:** copy the Mac DB to the NAS data dir before first run:
  `scp ~/Library/Application\ Support/com.kammerz.app/kammerz.db <nas>:/opt/kammerz/data/kammerz.db`. On boot, `seaql_migrations` is already fully populated → `Migrator::up` is a no-op → FK pragma is moot. Verify locally:
  ```bash
  cp ~/Library/Application\ Support/com.kammerz.app/kammerz.db /tmp/kz-existing.db 2>/dev/null \
    && DATABASE_URL="sqlite:/tmp/kz-existing.db?mode=rwc" target/release/kammerz & sleep 3 \
    && curl -s localhost:3001/api/auth/login -XPOST -H 'content-type: application/json' -d '{"password":"..."}' -c /tmp/j \
    && curl -s -b /tmp/j localhost:3001/api/rolls | head -c 200; kill %1
  ```
  Expected: existing rolls returned. (Skip if there's no local DB.)
- **Fresh start:** no copy. First boot runs all 20 migrations with FK OFF (seeding the user's gear via migrations 013–016). Verify a fresh DB seeds cameras: `GET /api/cameras` returns the seeded gear (already covered by `tests/cameras.rs`).

- [ ] **Step 2: Confirm schema parity vs the pre-port snapshot**

Run: `target/release/kammerz & sleep 3; sqlite3 ./kammerz.db ".schema" > /tmp/kammerz-schema-after.sql; kill %1; diff /tmp/kammerz-schema-before.sql /tmp/kammerz-schema-after.sql || echo "review diffs (session tables from tower-sessions are expected additions)"`
Expected: only additive differences (the `tower_sessions` table). Core schema identical.

### Task 16: Update CLAUDE.md, README, UI_DESIGN references

**Files:**
- Modify: `CLAUDE.md`
- Modify: `README.md`

- [ ] **Step 1: Rewrite the CLAUDE.md tech-stack + commands + architecture sections**

Key edits:
- Replace "Tauri 2 desktop app" framing with "axum + SvelteKit self-hosted web app."
- **Lift the preview-tools prohibition.** Replace the "DO NOT use preview tools … invoke() requires the native IPC bridge" note with: browser/Playwright verification is now valid; dev via `just dev` (axum :3001 + vite :5173 proxy); verify backend with `cargo test`, frontend with `bun run build`/`bun run check`, data via `sqlite3`.
- Update commands: `just dev`, `just build`, `cargo test`, `kammerz hash-password`.
- Update architecture: workspace (`entity`/`migration`/root binary), `routes/` replaces `commands/`, `auth/` module, `frontend/` location, the two-phase FK-pragma `db.rs`, single-password session auth, `request()` client replacing `invoke()`.
- Add: "**Auth:** single shared password via `KAMMERZ_PASSWORD_HASH`; open LAN-trust mode when unset. Routes under `(app)/` are guarded; `/login` is public."
- Keep all the SeaORM/migration/Svelte-pattern guidance — it's still accurate.

- [ ] **Step 2: README quickstart**

Document: dev (`just dev`), build (`just build`), generate password hash, `.env` setup, systemd deploy, the UniFi-VPN field-access note (configure on the gateway; app stays LAN-bound), and the data-carryover `scp` step.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "docs: update CLAUDE.md + README for the axum web architecture"
```

---

## PHASE 6 — End-to-end verification

### Task 17: Playwright smoke test of the served app

**Files:**
- Create: `frontend/tests/smoke.spec.ts`
- Modify: `frontend/package.json` (add `@playwright/test` + a `test:e2e` script)

> Now that it's a real browser app, Playwright works (the old Tauri IPC-bridge limitation is gone).

- [ ] **Step 1: Add Playwright + script**

`cd frontend && bun add -d @playwright/test && bunx playwright install chromium`. Add script `"test:e2e": "playwright test"`.

- [ ] **Step 2: Write the smoke test**

`frontend/tests/smoke.spec.ts`:
```typescript
import { test, expect } from '@playwright/test';

const BASE = process.env.E2E_BASE ?? 'http://localhost:3001';

test('login gate redirects then admits with correct password', async ({ page }) => {
	await page.goto(`${BASE}/cameras`);
	await expect(page).toHaveURL(/\/login/);
	await page.fill('input[type=password]', process.env.E2E_PASSWORD ?? 'secret');
	await page.click('button:has-text("Sign in")');
	await expect(page).toHaveURL(/\/cameras/);
	await expect(page.locator('body')).toContainText(/camera/i);
});
```

- [ ] **Step 3: Run against the release binary**

```bash
HASH=$(target/release/kammerz hash-password secret)
KAMMERZ_PASSWORD_HASH="$HASH" DATABASE_URL="sqlite:/tmp/kz-e2e.db?mode=rwc" target/release/kammerz &
sleep 3
cd frontend && E2E_PASSWORD=secret bunx playwright test; cd ..
kill %1; rm -f /tmp/kz-e2e.db
```
Expected: smoke test PASS.

- [ ] **Step 4: Manual parity walk-through (checklist)**

With the release binary running and logged in, click through each section and confirm it loads + a create/edit works: cameras, lenses, lens-mounts, film-stocks, labs, rolls (+ roll detail, shots, dev records), developments, search, stats, settings, quick-entry, import. Record any 4xx/5xx in the browser network panel and fix the offending route/wrapper.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "test(e2e): Playwright login + parity smoke against release binary"
```

### Task 18: Final gates + push

- [ ] **Step 1: Full check**

Run: `just check`
Expected: frontend check PASS, `cargo build` PASS, `cargo test` PASS.

- [ ] **Step 2: bd + git session-close**

```bash
git pull --rebase
bd dolt push
git push -u origin port-to-axum-web
git status   # up to date with origin
```

- [ ] **Step 3: Open the PR** (optional, via finishing-a-development-branch skill)

---

## Self-Review (author check against the spec)

- **Spec coverage:** Backend axum + rust-embed (Tasks 1,3,9,13) ✓ · SeaORM/SQLite reuse + migrate-on-startup (Tasks 1–3, db.rs) ✓ · single-password tower-sessions auth (Tasks 4–5,12) ✓ · `/api/*` per command (Tasks 6–8) ✓ · invoke→fetch in the 13 wrappers (Task 11) ✓ · `(app)` guard + `/login` (Task 12) ✓ · server-side Anthropic key (Task 8) ✓ · systemd + `.env` (Task 14) ✓ · UniFi-VPN = ops-only, no code (README note, Task 16) ✓ · lift preview-tools rule / Playwright now valid (Tasks 16,17) ✓ · data carryover (Task 15) ✓. **Out of scope honored:** no mobile/quick-entry redesign, no PWA, no multi-user — quick-entry is ported as-is, not redesigned.
- **Critical-detail coverage:** FK-OFF-during-migration preserved (db.rs, Task 3) ✓ · raw (no `{data}`) response envelope so wrapper return types are unchanged ✓ · `note_text` body-key rename called out (Task 11) ✓ · dead `mark_existing_migrations` dropped ✓.
- **Type consistency:** `request<T>(method, path, body?)`, `AppError`/`AppResult`, `RequireAuth`, `AppState`/`FromRef`, `friendly_err`, `AUTH_KEY` are used consistently across tasks.
- **Known follow-ups (separate specs, per the design):** mobile/field quick-entry UX; L3 offline PWA.
