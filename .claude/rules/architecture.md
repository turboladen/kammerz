# Architecture

## Data Flow

`Frontend (SvelteKit)` → `request()` (fetch) → `/api/*` axum route → `Service` → `SeaORM Entity` → `SQLite`

## Workspace layout

Cargo workspace (`Cargo.toml` `members = [".", "entity", "migration"]`):

- Root binary crate (`kammerz`) — the axum server
- `entity/` — SeaORM entity models (one file per table)
- `migration/` — SeaORM migration crate (schema + seed data), unchanged across the port

## Backend (Rust / axum)

- `src/main.rs` — Bootstrap: load `.env`, init DB, run migrations, build the session layer, mount routes, serve. Also handles the `hash-password` CLI subcommand. Embeds `frontend/build` via `rust-embed` (`#[folder = "frontend/build"]`) and serves it with an SPA fallback (`serve_spa`).
- `src/lib.rs` — `AppState { db, config }` + `FromRef` impls so handlers can extract `State<DatabaseConnection>` or `State<AppConfig>` directly. Also re-exports the testable lib modules (`compression`, `spa`, `extract`, `validate`).
- `src/config.rs` — `AppConfig::from_env()` reads env (`KAMMERZ_PASSWORD_HASH`, `ANTHROPIC_API_KEY`, `SECURE_COOKIES`, `BIND_ADDR`, `KAMMERZ_TRUST_PROXY`, `PORT`); returns `Result` so `main.rs` can fail fast with an operator message on a bad value.
- `src/db.rs` — Single-connection pool (max=min=1); FK-OFF → migrate → FK-ON sequence (see Database below).
- `src/error.rs` — `AppError` + `IntoResponse`; errors serialize as `{ "error": { "code", "message" } }`.
- `src/patch.rs` — `trim`/`trim_opt`/`double_option` helpers for partial-update DTOs.
- `src/auth/` — `password.rs` (argon2 hash/verify), `handlers.rs` (login/logout/me), `middleware.rs` (`RequireAuth` extractor + session helpers).
- `src/routes/` — One module per former Tauri command group (`cameras`, `lenses`, `lens_mounts`, `film_stocks`, `labs`, `rolls`, `shots`, `development`, `search`, `stats`, `settings`, `import`). `mod.rs::create_router()` merges all sub-routers + `/api/health` + auth routes. `friendly_err` lives here. **`routes/` replaces the old `commands/`** — same DTOs, same service calls; handlers take `RequireAuth` first, then `State`/`Path`/`Query`/`Json` extractors, return `Json<T>` / `StatusCode::NO_CONTENT` / `(StatusCode::CREATED, Json(id))`.
- `src/services/` — Business logic layer (CRUD + helpers), unchanged from the Tauri version.

## Auth

- Single shared password via `KAMMERZ_PASSWORD_HASH` (argon2). When **unset**, auth is OPEN (LAN-trust mode) and a startup warning is logged — fine for a trusted LAN, set the hash for any network-reachable deployment.
- `POST /api/auth/login` verifies the password and starts a tower-sessions session (cookie); `GET /api/auth/me` reports `{ authenticated, auth_required }`; `POST /api/auth/logout` flushes the session.
- `RequireAuth` extractor guards all business `/api` routes (401 when a hash is set and the session isn't authed; passes through in open mode).
- Frontend: routes under `frontend/src/routes/(app)/` are guarded by `(app)/+layout.ts` (redirects to `/login?next=…` when `auth_required && !authenticated`); `/login` is public.

## RPC → REST

Every former Tauri command maps to one route: reads `GET`, creates `POST` (→ `201` + id), updates `PUT` (→ `204`), deletes `DELETE` (→ `204`). `id` in the path, payloads in the JSON body. **Responses return the raw value (no `{data}` wrapper)** so the frontend wrapper return types match the old `invoke()` shapes.

## Frontend (SvelteKit)

- `frontend/` — the SvelteKit app (`package.json`, `vite.config.ts`, `svelte.config.js`, `src/`, `static/`). Builds to `frontend/build/`.
- `src/routes/(app)/` — Authenticated page components (file-based routing) behind the layout guard; `src/routes/login/` is the public login page.
- `src/lib/components/ui/` — Reusable UI components (Button, Input, Select, Dialog, etc.)
- `src/lib/components/layout/` — Layout components (Sidebar, PageHeader)
- `src/lib/api/` — Thin wrappers over the shared `request<T>(method, path, body?)` fetch helper in `client.ts` (sends cookies via `credentials: 'include'`, parses the `{error}` envelope, fires an unauthorized handler on 401). **`request()` replaces the old `invoke()`** — the wrapper signatures are otherwise unchanged.
- `src/lib/types/index.ts` — TypeScript interfaces for all entities
- Vite dev server proxies `/api` → `http://localhost:3002`; in production axum serves both the embedded SPA and the API.

## Database

- SQLite via SeaORM — all queries go through typed Rust entities
- Migrations run automatically via `Migrator::up()` at server startup (`db.rs::init`)
- `DATABASE_URL` selects the DB (dev default `sqlite:./kammerz.db?mode=rwc`). Carry over an existing Mac catalog by copying it to the configured path before first run — `seaql_migrations` is already populated so `Migrator::up` is a no-op.
- Single-connection pool (`max=min=1`) so the OFF→migrate→ON pragma sequence is deterministic and an in-memory test DB stays alive for the life of the pool. A single-user catalog never needs concurrent writers.
- SQLite pragmas: `journal_mode=WAL`, configurable `busy_timeout` (`SQLITE_BUSY_TIMEOUT_MS`, default 5000), `foreign_keys`. **Critical**: SQLx defaults `PRAGMA foreign_keys=ON` on SQLite connections — `db.rs` explicitly sets `foreign_keys=OFF` before `Migrator::up()` and re-enables `ON` after. Table-rebuild migrations (CREATE new → INSERT → DROP old → RENAME) trigger SQLite's implicit DELETE on DROP TABLE, which cascades through `ON DELETE CASCADE` (deleting junction rows) and `ON DELETE SET NULL` (NULLing FK columns).
- Junction table gotcha: Entity file is `camera_lens.rs` but the SQLite table name is `camera_lenses` (plural). Always check `#[sea_orm(table_name = "...")]` in entity files — don't guess from the filename.
- The only schema addition vs. a pre-port Tauri DB is the `tower_sessions` table (created by the session store's own migration). Core catalog schema is identical.
