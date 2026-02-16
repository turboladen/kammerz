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

## Phase 3: Backend Migration — SeaORM 🔲

Replace `tauri-plugin-sql` (JS calls raw SQL directly) with SeaORM (JS → Tauri command → Rust service → SQL). This adds type safety, proper migrations, and a service layer. Reference projects: `~/Development/projects/financier` and `~/Development/projects/fewd`.

### 3a. Rust scaffolding
- [ ] Add `sea-orm` and `sea-orm-migration` dependencies to `src-tauri/Cargo.toml`
- [ ] Create `migration/` crate alongside `src-tauri/` with `Migrator` struct
- [ ] Port existing `001_initial_schema.sql` into a SeaORM migration (`m{date}_001_initial_schema.rs`)
- [ ] Initialize DB in `lib.rs` setup hook: connect to SQLite, run migrations, store `DatabaseConnection` in `AppState`
- [ ] Set SQLite pragmas (`journal_mode=WAL`, `busy_timeout=5000`)

### 3b. Entities & services
- [ ] Define SeaORM entities in `src-tauri/src/entities/` (one file per table, `DeriveEntityModel` + `Serialize`/`Deserialize`)
  - `camera.rs`, `camera_maintenance.rs`, `lens.rs`, `camera_lens.rs`
  - `film_stock.rs`, `lab.rs`, `roll.rs`, `shot.rs`, `shot_lens.rs`
  - `development_lab.rs`, `development_self.rs`, `dev_stage.rs`
- [ ] Define entity relations (`Relation` enum, `Related` impls)
- [ ] Create service layer in `src-tauri/src/services/` (one struct per entity with CRUD methods)
  - e.g., `CameraService::get_all()`, `::get_by_id()`, `::create()`, `::update()`, `::delete()`
  - Distinct-value helpers (for ComboInput): `::distinct_brands()`, `::distinct_vendors()`, etc.
  - Roll queries with joined camera/film stock data (replaces current `listRollsWithDetails` SQL join)

### 3c. Tauri commands
- [ ] Create `src-tauri/src/commands/` modules with `#[tauri::command]` functions
  - Commands receive `State<AppState>`, delegate to services, return `Result<T, String>`
  - DTOs for request payloads (e.g., `CreateCameraDto`, `UpdateRollDto`)
- [ ] Register all commands in `lib.rs` `invoke_handler![]`
- [ ] Update Tauri capabilities — remove `sql:default` and `sql:allow-execute` (no longer needed)

### 3d. Frontend migration
- [ ] Create `src/lib/api/` layer — thin wrappers around `invoke()` from `@tauri-apps/api/core`
  - e.g., `listCameras()` → `invoke<Camera[]>("list_cameras")`
  - One file per entity matching current `src/lib/db/` structure
- [ ] Swap all page imports from `$lib/db/*` to `$lib/api/*`
- [ ] Remove `tauri-plugin-sql` dependency and old `src/lib/db/` files
- [ ] Verify all existing functionality works end-to-end

### 3e. Verification
- [ ] `bun run build` passes (frontend compiles)
- [ ] `cargo build` passes (Rust compiles)
- [ ] All CRUD operations work in the running app (create, read, update, delete for each entity)
- [ ] ComboInput autocomplete still populated
- [ ] Film stock format-aware ordering still works on New Roll page
- [ ] Dashboard stats and "needs attention" alerts still work

## Phase 4: Shot Entry & Quick Entry 🔲

Per-frame metadata logging and a streamlined "notes to data" workflow.

- [ ] **Shot entry UI** on roll detail page — add/edit/delete individual frames
  - Frame number, aperture, shutter speed, date, location, GPS, notes
  - Lens selection per shot (uses `shot_lenses` junction table)
  - Inline list of shots on the roll detail page
- [ ] **Quick Entry page** — bulk entry mode for transferring handwritten notes into structured shot data
  - Minimal UI optimized for speed: frame number + key fields, tab through
  - Auto-advance to next frame after save
- [ ] Frame count validation (warn if shots exceed roll's frame count)
- [ ] Camera-lens associations — UI for linking which lenses are compatible with which cameras
  - Uses existing `camera_lenses` junction table (schema already in place)
  - Filter lens dropdown on shot entry to lenses linked to the roll's camera

## Phase 5: Development Tracking 🔲

Record how each roll was developed, whether at a lab or self-developed.

- [ ] **Lab development** — track drop-off/pickup dates, cost, and which lab
  - Link to labs catalog
  - Auto-advance roll status to `at-lab` → `developed` based on dates
- [ ] **Self development** — full chemistry tracking
  - Developer, fixer, stop bath, wetting agent, clearing agent
  - Dilution ratios, temperature, agitation notes
  - `dev_stages` step-by-step timing (e.g., "Pre-soak 1:00", "Developer 8:30", "Stop 0:30")
- [ ] Development section on roll detail page (replaces or augments current status section)
- [ ] Cost tracking summary — development costs across rolls

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
| `cameras` | 🔲 Phase 3 | ✅ |
| `camera_maintenance` | 🔲 Phase 3 | ✅ |
| `lenses` | 🔲 Phase 3 | ✅ |
| `camera_lenses` | 🔲 Phase 3 | 🔲 Phase 4 |
| `film_stocks` | 🔲 Phase 3 | ✅ |
| `labs` | 🔲 Phase 3 | ✅ |
| `rolls` | 🔲 Phase 3 | ✅ |
| `shots` | 🔲 Phase 3 | 🔲 Phase 4 |
| `shot_lenses` | 🔲 Phase 3 | 🔲 Phase 4 |
| `development_lab` | 🔲 Phase 3 | 🔲 Phase 5 |
| `development_self` | 🔲 Phase 3 | 🔲 Phase 5 |
| `dev_stages` | 🔲 Phase 3 | 🔲 Phase 5 |

## Reference Projects

| Project | Path | Pattern | Useful for |
|---|---|---|---|
| **financier** | `~/Development/projects/financier` | Tauri 2 + SeaORM + React | Tauri command registration, AppState, DB init, migration cleanup |
| **fewd** | `~/Development/projects/fewd` | Axum + SeaORM (web server) | Service layer patterns, entity definitions, error handling |
