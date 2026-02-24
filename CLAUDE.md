# Kammerz

Film photography catalog desktop app built with Tauri 2 + SvelteKit + SQLite.

## Tech Stack

- **Tauri 2** (v2.10.0) — native macOS app using system WebKit webview
- **SvelteKit** with **Svelte 5** runes (`$state`, `$derived`, `$effect`, `$props`, `$bindable`)
- **Bun** as package manager and JS runtime
- **SQLite** via **SeaORM 1.1** (Rust ORM) — typed entities, services, and migrations
- **Tailwind CSS 4** with `@tailwindcss/vite` plugin and custom dark theme via `@theme`
- **adapter-static** for SvelteKit (Tauri has no server — serves static files)

## Commands

- `bun run tauri dev` — Run the app in development mode (Tauri + Vite)
- `bun run tauri build` — Build the production .app bundle
- `bun run build` — Build SvelteKit frontend only (useful for quick compile checks)
- `cargo build` — Build Rust backend only (in `src-tauri/`)
- `bun run dev` — Run Vite dev server only (no Tauri backend — DB calls will fail)
- **DO NOT use preview tools (preview_start/preview_screenshot/etc.) to validate changes.** `invoke()` requires the native IPC bridge in Tauri's WebKit webview — pages will render blank. Verify backend via `cargo build` + Tauri dev server logs. Verify frontend markup via `bun run build` (compile check). Verify data via `sqlite3` queries against `~/Library/Application Support/com.kammerz.app/kammerz.db`.
- If `bun run tauri dev` fails with "Port 1420 is already in use", kill the orphaned Vite process: `lsof -ti:1420 | xargs kill -9`. Happens when a previous Tauri dev session was interrupted without cleanly killing child processes.

## Architecture

### Data Flow

`Frontend (SvelteKit)` → `invoke()` → `Tauri Command` → `Service` → `SeaORM Entity` → `SQLite`

### Frontend (SvelteKit)

- `src/routes/` — Page components (SvelteKit file-based routing)
- `src/lib/components/ui/` — Reusable UI components (Button, Input, Select, Dialog, etc.)
- `src/lib/components/layout/` — Layout components (Sidebar, PageHeader)
- `src/lib/api/` — Thin wrappers around `invoke()` from `@tauri-apps/api/core`
- `src/lib/types/index.ts` — TypeScript interfaces for all entities
- `ssr = false` and `prerender = false` in root layout (required for Tauri)

### Backend (Rust/Tauri)

- `src-tauri/src/lib.rs` — Tauri app setup, AppState, command registration
- `src-tauri/src/db.rs` — Database connection, pragmas, migration runner
- `src-tauri/src/entities/` — SeaORM entity models (one file per table, 12 total)
- `src-tauri/src/services/` — Business logic layer (CRUD + helpers, 5 files)
- `src-tauri/src/commands/` — `#[tauri::command]` handlers with DTOs (5 files)
- `src-tauri/migration/` — SeaORM migration crate (schema + seed data)
- `src-tauri/capabilities/default.json` — Tauri 2 permission grants

### Database

- SQLite via SeaORM — all queries go through typed Rust entities
- Migrations run automatically via `Migrator::up()` on app start
- Existing databases (from old tauri-plugin-sql) auto-detected and migration table bridged
- SQLite pragmas: `journal_mode=WAL`, `busy_timeout=5000`, `foreign_keys=ON`. **Critical**: SQLx sets `PRAGMA foreign_keys=ON` by default on SQLite connections — `db.rs` must explicitly set `foreign_keys=OFF` before `Migrator::up()` and re-enable after. Table-rebuild migrations (CREATE new → INSERT → DROP old → RENAME) trigger SQLite's implicit DELETE on DROP TABLE, which cascades through `ON DELETE CASCADE` (deleting junction rows) and `ON DELETE SET NULL` (NULLing FK columns). Performance pragmas are safe before migrations.
- Junction table gotcha: Entity file is `camera_lens.rs` but the SQLite table name is `camera_lenses` (plural). Always check `#[sea_orm(table_name = "...")]` in entity files — don't guess from the filename.
- DB location (macOS): `~/Library/Application Support/com.kammerz.app/kammerz.db` — can query directly with `sqlite3` for debugging.

## Important Conventions

### UX Rules
- **Always confirm destructive actions.** Never delete data without user confirmation.
- Back navigation: Detail pages use `PageHeader`'s `backHref`/`backLabel` props for consistent back links. Cross-entity links (e.g., developments→roll, dashboard→roll, search→roll/camera) pass `?from=<source>` query param; detail pages read this via `$page.url.searchParams.get('from')` and map it to the correct back route. See `backRoutes` map in `rolls/[id]/+page.svelte`.
- Owned/Sold filtering: List pages with `date_sold` fields (cameras, lenses) use client-side All/Owned/Sold tab buttons with a `$derived()` filter. No backend changes needed to add this to a new list page.
- Fixed-lens cameras: Structural invariant — camera creation with "Fixed Lens" mount MUST always call `createCameraWithLens()` (never plain `createCamera()`). Show read-only lens indicators everywhere — lens list cards ("Fixed on [Camera]"), lens edit dialog (accent banner), roll default lens (locked text), shot lens dropdown (read-only), Quick Entry (locked text). Camera detail page shows "Built-in Lens" section (no unlink/link/default-change controls). Camera edit locks the mount field to read-only. Detect via mount name: `lensMounts.find(m => m.id === mountId)?.name === 'Fixed Lens'` or `lensMounts.some(m => m.id === mountId && m.name === 'Fixed Lens')` — never hardcode mount IDs.
- Shot lens defaults: Smart cascade — fixed lens (auto-locked) > last-used lens on roll > `roll.lens_id` (roll default) > `camera.default_lens_id` (camera default) > empty.
- Shot date defaults: Smart cascade — last shot's date on roll > `roll.date_loaded` (first shot) > empty. Date persists as a session default across "Save & Next".
- Development auto-prompt: Moving status to "at-lab" auto-opens lab dev dialog; "developing" auto-opens self dev dialog (only if neither dev record exists). Lab and self dev are mutually exclusive — UI hides "+ Lab" / "+ Self" buttons once one exists.
- Data-driven status sync: Roll status auto-advances and auto-reverts based on related data, handled transactionally in backend commands via `RollService::auto_sync_status()`. Rules: first shot added → `loaded→shooting`; lab dev created → `→at-lab`; self dev created → `→developing`; all shots deleted → `shooting/shot→loaded`; lab dev deleted → `at-lab/lab-done→shot`; self dev deleted → `developing/developed→shot`. Status beyond a data type's range is not affected (e.g., deleting a dev record at `scanned` stays at `scanned`). Manual backward moves (chevron clicks) still require `ConfirmDialog` — the confirmation exists for when the user intentionally contradicts the data state. Never use frontend `$effect` for status sync — the backend owns this logic.
- Shot dialog "Save & Next": Keeps dialog open after save, resets per-shot fields (aperture, shutter, notes), preserves session defaults (date, location, lens), auto-suggests next frame number. Only shown in add mode (not edit).
- Dashboard roll sections: "In the Field" shows `loaded`/`shooting` rolls (in a camera). "In the Darkroom" shows `shot`/`at-lab`/`lab-done`/`developing`/`developed`/`scanned` rolls (post-shooting pipeline, sorted by status progression). "Needs Attention" shows rolls with `!camera_id` (excluding archived). All non-archived rolls must appear in at least one clickable section.

### Svelte 5 Patterns
- Use `$state()`, `$derived()`, `$effect()`, `$props()`, `$bindable()` — no legacy `let` reactivity.
- Use `onclick={handler}` on buttons instead of `<form onsubmit>`. Form submission events don't work reliably in Tauri's WebKit webview inside conditional Svelte blocks.
- Button component passes `onclick` via `{...rest}` spread to the native `<button>` element.
- Detail page edit mode: When a page has view/edit toggle, maintain parallel `$derived` vars — e.g., `selectedCamera` (from saved `roll.camera_id`) for shot defaults vs `editSelectedCamera` (from `editCameraId` form state) for edit-mode film stock/lens filtering.

### Tauri 2 / SeaORM Patterns
- Commands receive `State<'_, AppState>`, delegate to services, return `Result<T, String>`
- Services are static async methods on unit structs (e.g., `CameraService::list_all(&db)`)
- Entities use `String` for timestamps (SQLite TEXT), `Option<T>` for nullable fields
- For joined queries (e.g., rolls with camera/film stock), use `#[derive(FromQueryResult)]` with raw SQL
- For junction table data (e.g., shot↔lens), prefer batch queries per parent (e.g., `get_lenses_for_roll_shots(roll_id)`) over per-row queries. Avoids N+1 IPC round-trips through Tauri's `invoke()`.
- DTOs in command files handle create/update payloads; services work with `ActiveModel` directly
- Transaction trait imports: When using SeaORM entity operations directly inside a `transaction` closure (instead of via service methods), additional traits must be in scope: `ActiveModelTrait` (for `.insert()`, `.update()` on models), `ColumnTrait` + `QueryFilter` (for `.filter(...eq(...))` queries), `PaginatorTrait` (for `.count()`). The `create_shot` and `delete_shot` commands show the full import set.
- Changes to `src-tauri/` files (Rust, capabilities, Cargo.toml) require Tauri to recompile
- Migration raw SQL gotcha: `execute_unprepared()` auto-commits each statement. If a migration fails midway, partial data persists but the migration isn't recorded in `seaql_migrations` — so it re-runs on next start, creating duplicates. Use `INSERT OR IGNORE` for data, `IF NOT EXISTS` for `CREATE INDEX`/`CREATE TABLE`, and `DROP ... IF EXISTS` before recreating objects. For tables without unique constraints (cameras, lenses), failures require manual cleanup via `sqlite3`.
- Seed migration pattern: Use subqueries for FK resolution (`(SELECT id FROM lens_mounts WHERE name = 'Nikon F')`) instead of hardcoded IDs — IDs vary across environments and are fragile across migration reorders.
- Fixed-lens seed pattern: Each fixed-lens camera requires 4 SQL statements in order: INSERT camera → INSERT lens → INSERT camera_lenses junction → UPDATE camera.default_lens_id. See migration 013 for the template.
- Table-rebuild migration safety: Any migration that uses DROP TABLE on a parent table (cameras, lenses, etc.) will cascade-delete rows in child tables (camera_lenses, shot_lenses) if FK enforcement is on. The `db.rs` fix (`PRAGMA foreign_keys=OFF` before migrations) prevents this, but if writing new table-rebuild migrations, always verify the FK pragma state. Migration 020 is a repair migration for data lost before the pragma fix was in place.
- Batch child merge pattern: When a list endpoint needs parent rows + child collections (e.g., developments + stages), fetch parents first, collect IDs, batch-fetch children via `IN (...)`, merge via `HashMap<parent_id, Vec<Child>>` with `.remove()` (not `.get()`) to avoid cloning. See `list_all_self_developments` in `commands/development.rs`.
- DeriveActiveEnum pattern: Columns with constrained string values (status, format, type) use `#[derive(DeriveActiveEnum)]` with `#[sea_orm(string_value = "...")]` + `#[serde(rename = "...")]`. Enums defined in entity files alongside the Model (e.g., `RollStatus` in `roll.rs`). Raw SQL `FromQueryResult` structs can use these enum types directly — SeaORM's `TryGetable` handles deserialization. When adding new enum variants, update both the Rust enum and the TypeScript union type in `src/lib/types/index.ts`.
- Error helper: `commands/mod.rs::friendly_err(context, error)` maps DB constraint errors to user-friendly messages. `context` is a noun ("roll", "camera", "film stock"). Uses `raw.find()` (not `strip_prefix`) for constraint detection because SeaORM wraps errors with context prefixes. Apply to all create/update/delete `map_err` closures — not read-only queries (reads never trigger constraint errors).
- Composite commands: When a detail page needs data from multiple tables, use a single `#[tauri::command]` that aggregates all queries (e.g., `get_roll_detail` returns roll + shots + shot_lenses + devs + stages). Reduces IPC round-trips through Tauri's `invoke()` bridge.
- Raw SQL with `find_by_statement`: Prefer `SELECT *` over explicit column lists — SeaORM's `FromQueryResult` maps by column name, not position, so `SELECT *` stays in sync if entity fields change. Only use raw SQL when SeaORM's query builder can't express the query (e.g., `ORDER BY CAST(col AS INTEGER)`).

### Camera Format Dropdown
- Camera type options: `SLR`, `rangefinder`, `TLR`, `point-and-shoot`, `box` (Box Camera), `instant`, `view` (View/Field Camera). Defined in `typeOptions` arrays in both `cameras/+page.svelte` and `cameras/[id]/+page.svelte` — keep them in sync.
- Includes generic "Medium Format" and "Large Format" options for cameras that support multiple backs (e.g., Mamiya RB67).
- Format labels use "Medium Format: 6x6" style (not "6x6 (Medium Format)").
- Camera format → film stock format mapping: `35mm`→`135`, all medium format variants (`6x4.5`–`6x9`)→`120`, large format sizes map directly (`4x5`→`4x5`, etc.), `instant`→`instant`. Don't filter out non-matching formats — only reorder (cameras can use different backs).
- 120 film stocks have `exposure_count: NULL` by design — frame count depends on back size (6×4.5=15, 6×6=12, 6×7=10, 6×8=9, 6×9=8). Sheet film stocks (4x5, 5x7, 8x10) have `exposure_count: 1` (one sheet per holder side) — auto-fill uses `> 1` guard to skip these. `frameCountHint` derived state shows format-specific guidance for both 120 and sheet film when frame count is empty.
- Generic "Large Format" lens mount was removed (migration 009). LF cameras should use a specific shutter mount (Copal #0/#1/#3, Compur #0/#1/#3, Barrel Mount). Cross-mount compatibility within the LF family is handled by `isLargeFormatMount()` in `$lib/utils/lens.ts`.

### Component Patterns
- List pages use `ListToolbar` (search + group-by + sort) with `$bindable()` props. Pipeline: primary filter (ownership/status/type tabs) → `filterBySearch()` → sort → `groupItems()` — all via `$derived` chain. Utilities in `src/lib/utils/list.ts`.
- `GroupHeader` renders the ledger-line group label. Uses `{#if label}` guard so `groupBy === 'none'` (empty-string key) renders nothing.
- Collection Cards (cameras, lenses): `grid-cols-[repeat(auto-fill,minmax(260px,1fr))]` card grid for short scannable data. Lenses use `minmax(280px,1fr)` for edit/delete buttons.
- List Rows (rolls, film stocks): Full-width `px-4 py-2.5` rows with `gap-1.5` for items with wide relational data.
- `totalCount` in ListToolbar: Pass post-primary-filter count (e.g., `afterOwnerFilter.length`), not `items.length` — the "X of Y" denominator should reflect the active tab scope.
- Empty state on list pages: Three-branch pattern — `resultCount === 0 && items.length === 0` → `EmptyState` with icon + CTA; `resultCount === 0` → "No matches" text; else → render items.
- Dialog Cancel buttons must call `resetForm()` to clear stale form data (same as the success path).
- `ComboInput` dropdown options use `onmousedown` (not `onclick`) to beat the blur/click race condition.
- `Select` options support an optional `disabled` property (used for visual dividers like `── Other formats ──`).
- `Select` uses explicit `h-[38px]` to match `Input`/`DateInput` height — WebKit renders `<select>` shorter than `<input>` with identical padding classes.
- Use `$derived.by(() => { ... })` when derived state needs multi-line logic; `$derived(expr)` for one-liners.
- Use `{#snippet name(params)}` / `{@render name(params)}` for reusable template blocks within a single component — avoids duplicating markup without extracting a separate component file. See `rollCard` snippet in `+page.svelte` (Dashboard).
- Always use the `<Badge>` component for roll statuses — never inline status pills with raw classes.
- Wrap page content sections in `<FadeIn>` with staggered `delay` props (typically 50ms increments) for consistent entrance animations.
- FadeIn stacking context: CSS `animation` with `transform`/`opacity` creates a stacking context + containing block, trapping `position: fixed` children (e.g., Dialogs). FadeIn strips its animation class via `onanimationend` to clear this after the entrance plays. Never wrap a component that renders its own Dialog inside a persistent animation/transform.
- Section headers use the ledger-line pattern: `text-xs font-semibold uppercase tracking-wider text-text-faint` with either a rule line (`<div class="flex-1 border-b border-border-subtle">`) or `justify-between` for headers with action buttons. Never use `text-sm font-semibold text-text-muted`.
- Card hover borders always use `hover:border-accent/40` — never other opacities like `/30`.
- Roll status metadata (labels, colors, CSS classes) is defined in `src/lib/utils/status.ts`. Always import from there — never define inline status maps in page components. Use `getStatusColor(status)` for typed lookups or `getStatusColorSafe(label)` for untyped strings from backend queries.
- Roll status flows: `status.ts` exports path-specific arrays (`labFlow`, `selfFlow`, `undecidedFlow`) and `allStatusOrder` (union of all statuses). The deprecated `export const statusOrder = allStatusOrder` alias exists for backward compatibility — don't remove it, but don't use it in new code. Use `getFlowForPath(devPath)` instead.
- Roll status progression: Path-aware chevron bar shows only statuses relevant to the roll's development path. Three flows defined in `status.ts`: `labFlow` (loaded→shooting→shot→at-lab→lab-done→scanned→archived), `selfFlow` (loaded→shooting→shot→developing→developed→scanned→archived), `undecidedFlow` (loaded→shooting→shot→scanned→archived with placeholder chevrons in the gap). Path determined by `getDevPath(status, hasLabDev, hasSelfDev)` — dev record presence takes priority, then status value. Chevron styles: current (`bg-accent`), past (`bg-accent/10`), future (`bg-surface-overlay`). Forward status changes are instant (backend is fully permissive); backward moves require `ConfirmDialog`. See `handleStatusClick()` + `devPath` + `statusFlow` in `rolls/[id]/+page.svelte`.
- Lens naming: Lenses use `brand` + `model` (paralleling cameras). The `model` field (formerly `name_on_lens`) should NOT include the brand. `lensDisplayName()` in `$lib/utils/lens.ts` always returns `brand + model`, with a `startsWith` guard to avoid doubling when model already contains the brand. Fallback when model is empty: `brand + focal_length + max_aperture`.
- Lens dropdowns: Always use `buildLensOptions()` from `$lib/utils/lens.ts` — handles mount-compatibility sorting with dividers and automatic disambiguation of duplicate lenses. Also see `buildMountOptions()` for mount dropdowns grouped by format family.
- Camera dropdowns: Use `buildCameraLabels()` from `$lib/utils/disambiguate.ts` to get a `Map<id, label>` that auto-disambiguates duplicate cameras. When two cameras share the same `brand + model`, the label appends `(S/N xxxxx)` if serial exists or `(Copy N)` by creation order. Single instances stay clean.
- Dialog component uses flex column layout with `max-h-[85vh]` and `overflow-y-auto` on content. When adding fields to dialogs (e.g., inline lens creation), scrolling is already handled.

### Error Handling
- Frontend `invoke()` calls return promises that reject on error. Wrap in try/catch with user-visible error display.
- Always validate required fields client-side before `invoke()` calls (brand, model, mount, etc.). Show inline `error` state text — don't rely on backend DB constraint errors which are opaque to users.

### UI Design
- Follow the design system in `UI_DESIGN.md` — colors, typography, component styling, layout patterns, and design principles.
- All colors use CSS custom properties defined in `src/app.css` via Tailwind's `@theme`. Never use raw hex colors.
- Fonts: DM Sans (UI), IBM Plex Mono (data), Instrument Serif (display). Self-hosted in `static/fonts/`.
- Keep `UI_DESIGN.md` updated when design decisions change.

## Reference

- Another Tauri 2 + SQLite project by the same author: `~/Development/projects/financier` (same SeaORM patterns)
- `UI_DESIGN.md` documents the visual design system (colors, typography, components, layout)
- `IMPLEMENTATION_PLAN.md` tracks phase-by-phase development progress
