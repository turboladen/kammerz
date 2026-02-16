# Kamerz — Implementation Plan

Tracking development progress for the film photography catalog app.

---

## Phase 1: Foundation & Gear Catalog ✅

Core CRUD pages for managing photography equipment, plus the database schema for the entire app.

- [x] SQLite database schema (all 12 tables, indexes, constraints)
- [x] TypeScript interfaces for all entities
- [x] Database access layer (`src/lib/db/`) with CRUD operations
- [x] App shell: sidebar navigation, page header, dark theme
- [x] **Cameras** — list with owned/sold filters, add dialog, detail page with edit
- [x] **Lenses** — list, add/edit/delete with full optical specs (mount, aperture range, filter threads)
- [x] **Film Stocks** — catalog with type filters (color neg, B&W, slide) and format filters (135, 120, 4x5+)
- [x] **Labs** — list, add/edit/delete with location and website
- [x] Camera maintenance history (CLA, repair, cleaning, modification) with cost tracking
- [x] Reusable UI components: Button, Input, Select, Textarea, Dialog, Badge, EmptyState
- [x] Tauri 2 permission fix (`sql:allow-execute` for write operations)
- [x] `onclick` pattern instead of `<form onsubmit>` (WebKit webview compatibility)

## Phase 2: Rolls & Workflow ✅

Roll lifecycle management from loading film to archiving, plus UX polish across the app.

- [x] **Rolls** — list with status filters, create new roll, detail page
- [x] Roll ID auto-suggestion (YYMMDD-N format)
- [x] Status progression workflow (loaded → shooting → shot → at-lab → developing → developed → scanned → archived)
- [x] Camera assignment on rolls (with camera dropdown)
- [x] Film stock selection with format-aware ordering (matching stocks prioritized by camera format, divider, then rest)
- [x] **Dashboard** — roll counts by status, "needs attention" alerts (missing camera, waiting at lab)
- [x] Rolls-per-camera list on camera detail page
- [x] `ConfirmDialog` component — all destructive actions require confirmation
- [x] `ComboInput` component — autocomplete for brands, vendors, lens systems, maintenance providers
- [x] Back navigation on detail pages (PageHeader `backHref`/`backLabel`)
- [x] Date Sold on Add Camera form (for retroactive data entry)

## Phase 3: Backend Migration — SeaORM ✅

Replaced `tauri-plugin-sql` (JS calls raw SQL directly) with SeaORM (JS → Tauri command → Rust service → SQL). This adds type safety, proper migrations, and a service layer.

### 3a. Rust scaffolding
- [x] Add `sea-orm` and `sea-orm-migration` dependencies to `src-tauri/Cargo.toml`
- [x] Create `migration/` crate alongside `src-tauri/` with `Migrator` struct
- [x] Port existing `001_initial_schema.sql` into a SeaORM migration (`m20250101_000001_initial_schema.rs`)
- [x] Port seed data into `m20250101_000002_seed_film_stocks.rs`
- [x] Initialize DB in `lib.rs` setup hook: connect to SQLite, run migrations, store `DatabaseConnection` in `AppState`
- [x] Set SQLite pragmas (`journal_mode=WAL`, `busy_timeout=5000`)
- [x] Detect-and-skip for existing databases (bridge `_sqlx_migrations` → `seaql_migrations`)

### 3b. Entities & services
- [x] Define SeaORM entities in `src-tauri/src/entities/` (12 files, `DeriveEntityModel` + `Serialize`/`Deserialize`)
- [x] Define entity relations (`Relation` enum, `Related` impls)
- [x] Create service layer in `src-tauri/src/services/` (5 files with CRUD methods)
- [x] Distinct-value helpers (for ComboInput): `distinct_brands()`, `distinct_vendors()`, etc.
- [x] Roll queries with joined camera/film stock data (`RollWithDetails` via `FromQueryResult`)

### 3c. Tauri commands
- [x] Create `src-tauri/src/commands/` modules (5 files) with `#[tauri::command]` functions + DTOs
- [x] Register all ~35 commands in `lib.rs` `invoke_handler![]`
- [x] Update Tauri capabilities — removed `sql:default` and `sql:allow-execute`

### 3d. Frontend migration
- [x] Create `src/lib/api/` layer — thin wrappers around `invoke()` (5 files)
- [x] Swap all 9 route page imports from `$lib/db/*` to `$lib/api/*`
- [x] Remove `@tauri-apps/plugin-sql` dependency from `package.json`

### 3e. Verification
- [x] `bun run build` passes (frontend compiles)
- [x] `cargo build` passes (Rust compiles)
- [ ] All CRUD operations work in the running app (manual end-to-end testing needed)
- [ ] ComboInput autocomplete still populated
- [ ] Film stock format-aware ordering still works on New Roll page
- [ ] Dashboard stats and "needs attention" alerts still work

## Phase 4: Shot Entry & Quick Entry ✅

Per-frame metadata logging and a streamlined "notes to data" workflow.

- [x] **Shot service** (`src-tauri/src/services/shot_service.rs`) — CRUD, shot-lens junction, frame suggestion
- [x] **Shot commands** (`src-tauri/src/commands/shots.rs`) — 8 Tauri commands with DTOs
- [x] **Frontend API** (`src/lib/api/shots.ts`) — invoke wrappers for all shot commands
- [x] **Shared lens utility** (`src/lib/utils/lens.ts`) — `lensDisplayName()` used across pages
- [x] **Camera-lens associations** — UI on camera detail page for linking/unlinking lenses
- [x] **Shot entry UI** on roll detail page — add/edit/delete individual frames
  - Frame number, aperture, shutter speed, date, location, notes
  - Lens selection per shot via checkboxes (camera-linked lenses first)
  - Frame progress bar with overcount warning
- [x] **Quick Entry page** — rapid shot logging with roll selector
  - Auto-advance frame number after save, keep lens selection
  - ⌘+Enter keyboard shortcut, focus management
  - Recent shots preview (reverse chronological)
- [x] Frame count validation (warning banner when shots exceed roll's frame count)

## Phase 5: Development Tracking ✅

Record how each roll was developed, whether at a lab or self-developed.

- [x] **Development service** (`src-tauri/src/services/development_service.rs`) — lab dev, self dev, stages CRUD
- [x] **Development commands** (`src-tauri/src/commands/development.rs`) — 9 Tauri commands with DTOs
- [x] **Frontend API** (`src/lib/api/development.ts`) — invoke wrappers for all development commands
- [x] **Lab development** — track drop-off/pickup dates, cost, and which lab
  - Lab selector from labs catalog
  - Date fields, cost tracking, notes
- [x] **Self development** — full chemistry tracking
  - Developer, fixer, stop bath, wetting agent, clearing agent
  - Dilution ratios, temperature, agitation notes
  - Inline-editable dev stages with mm:ss duration and reorder (up/down)
- [x] **Development section** on roll detail page between Status and Shots
- [x] **Auto-prompt**: Status change to `at-lab` → opens lab dev dialog; `developing` → opens self dev dialog
- [x] **Exclusive development**: One dev record per roll (lab or self, not both)

## Phase 6: Search, Import & Polish 🔲

Cross-catalog search, data portability, and fit-and-finish improvements.

- [ ] **Search page** — full-text search across cameras, lenses, film stocks, rolls, shots, and notes
- [ ] Data import from CSV/JSON (for migrating from spreadsheets or other tools)
- [ ] Data export (backup entire catalog to JSON)
- [ ] Bulk operations (e.g., mark multiple rolls as a batch)
- [ ] Statistics and insights (most-used film stock, shots per camera, rolls per month)
- [ ] Image/scan attachment support (link scanned images to rolls or individual shots)
- [ ] Print/share roll summaries

---

## Schema Status

All 12 database tables exist. Phase 3 ports the schema to SeaORM migrations and entities. No new tables are needed for Phases 4–5.

| Table | SeaORM Entity | UI Built |
|---|---|---|
| `cameras` | ✅ | ✅ |
| `camera_maintenance` | ✅ | ✅ |
| `lenses` | ✅ | ✅ |
| `camera_lenses` | ✅ | ✅ |
| `film_stocks` | ✅ | ✅ |
| `labs` | ✅ | ✅ |
| `rolls` | ✅ | ✅ |
| `shots` | ✅ | ✅ |
| `shot_lenses` | ✅ | ✅ |
| `development_lab` | ✅ | ✅ |
| `development_self` | ✅ | ✅ |
| `dev_stages` | ✅ | ✅ |

## Reference Projects

| Project | Path | Pattern | Useful for |
|---|---|---|---|
| **financier** | `~/Development/projects/financier` | Tauri 2 + SeaORM + React | Tauri command registration, AppState, DB init, migration cleanup |
| **fewd** | `~/Development/projects/fewd` | Axum + SeaORM (web server) | Service layer patterns, entity definitions, error handling |
