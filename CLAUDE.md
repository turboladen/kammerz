# Kammerz

Film photography catalog — a self-hosted web app built with axum + SvelteKit + SQLite. Runs as a single binary on a LAN (NAS/home server), reachable from the field over the UniFi-gateway VPN. Single shared password.

## Tech Stack

- **axum 0.8** (Rust) — HTTP server exposing `/api/*` JSON endpoints and serving the embedded SvelteKit build
- **SvelteKit** with **Svelte 5** runes (`$state`, `$derived`, `$effect`, `$props`, `$bindable`)
- **Bun** as package manager and JS runtime
- **SQLite** via **SeaORM 1.1** (Rust ORM) — typed entities, services, and migrations
- **tower-sessions** (SQLite-backed) + **argon2** — single-password session auth
- **rust-embed** — the SvelteKit `frontend/build` is baked into the release binary (SPA fallback in `main.rs`)
- **Tailwind CSS 4** with `@tailwindcss/vite` plugin and custom dark theme via `@theme`
- **adapter-static** for SvelteKit (SPA — `ssr = false`, served as static assets by axum)

## Commands

- `just dev` — Run backend (axum on :3002) + frontend (Vite on :5273, proxies `/api` → :3002) together for development
- `just dev-backend` / `just dev-frontend` — Run either half alone
- `just build` — Production build: `frontend/build` (Vite) then `cargo build --release` (embeds it). Binary at `target/release/kammerz`
- `just check` — `cargo build` + `cargo test` + `bun run check` (svelte-check) + `bun run build`. All are hard gates. **Run this and ensure `bun run check` passes before opening a PR.**
- `cargo test -p kammerz` — Backend integration tests (in-memory SQLite, real migrations + seed)
- **CI** (`.github/workflows/ci.yml`) runs on every PR and push to `main`: a `backend` job (`cargo test`), a `frontend` job (`bun run check` + `bun run build`), and an `e2e` job (Playwright `smoke.spec.ts` against the release binary on :3002). The first two are required gates; mirror them locally with `just check`.
- `just ci` — Full local mirror of the GitHub Actions pipeline, all three jobs in order: `ci-backend` (`cargo build`/`test --locked`), `ci-frontend` (frozen `bun install` + check + build), `e2e` (Playwright smoke against the release binary on :3002, DB under /tmp). **GitHub Actions is currently unavailable on this account (billing) — run `just ci` and report its result as the PR gate instead.**
- `echo -n <pw> | kammerz hash-password` — Generate the argon2 hash for `KAMMERZ_PASSWORD_HASH`. **Reads the password from stdin, never argv** (argv leaks into shell history / `ps`). On a TTY it prompts with echo off.
- **Build wipes `frontend/build/`** — `bun run build` (and thus `just check` / `just build`) deletes the tracked `frontend/build/.gitkeep` (the adapter-static output dir is regenerated). Restore it before committing: `git checkout -- frontend/build/.gitkeep`. Don't commit its deletion.
- **Verification:** This is a normal browser app — browser/Playwright verification is valid. Run via `just dev` (axum :3002 + Vite :5273 proxy) and open `http://localhost:5273`, or build and run the release binary on :3002. Verify backend with `cargo test`, frontend markup/types with `bun run build` / `bun run check`, data with `sqlite3` queries against the configured `DATABASE_URL` (dev default `./kammerz.db`).

## Architecture

### Data Flow

`Frontend (SvelteKit)` → `request()` (fetch) → `/api/*` axum route → `Service` → `SeaORM Entity` → `SQLite`

### Workspace layout

Cargo workspace (`Cargo.toml` `members = [".", "entity", "migration"]`):

- Root binary crate (`kammerz`) — the axum server
- `entity/` — SeaORM entity models (one file per table)
- `migration/` — SeaORM migration crate (schema + seed data), unchanged across the port

### Backend (Rust / axum)

- `src/main.rs` — Bootstrap: load `.env`, init DB, run migrations, build the session layer, mount routes, serve. Also handles the `hash-password` CLI subcommand. Embeds `frontend/build` via `rust-embed` (`#[folder = "frontend/build"]`) and serves it with an SPA fallback (`serve_spa`).
- `src/lib.rs` — `AppState { db, config }` + `FromRef` impls so handlers can extract `State<DatabaseConnection>` or `State<AppConfig>` directly.
- `src/config.rs` — `AppConfig::from_env()` (`KAMMERZ_PASSWORD_HASH`, `ANTHROPIC_API_KEY`, `SECURE_COOKIES`).
- `src/db.rs` — Single-connection pool (max=min=1); FK-OFF → migrate → FK-ON sequence (see Database below).
- `src/error.rs` — `AppError` + `IntoResponse`; errors serialize as `{ "error": { "code", "message" } }`.
- `src/patch.rs` — `trim`/`trim_opt`/`double_option` helpers for partial-update DTOs.
- `src/auth/` — `password.rs` (argon2 hash/verify), `handlers.rs` (login/logout/me), `middleware.rs` (`RequireAuth` extractor + session helpers).
- `src/routes/` — One module per former Tauri command group (`cameras`, `lenses`, `lens_mounts`, `film_stocks`, `labs`, `rolls`, `shots`, `development`, `search`, `stats`, `settings`, `import`). `mod.rs::create_router()` merges all sub-routers + `/api/health` + auth routes. `friendly_err` lives here. **`routes/` replaces the old `commands/`** — same DTOs, same service calls; handlers take `RequireAuth` first, then `State`/`Path`/`Query`/`Json` extractors, return `Json<T>` / `StatusCode::NO_CONTENT` / `(StatusCode::CREATED, Json(id))`.
- `src/services/` — Business logic layer (CRUD + helpers), unchanged from the Tauri version.

### Auth

- Single shared password via `KAMMERZ_PASSWORD_HASH` (argon2). When **unset**, auth is OPEN (LAN-trust mode) and a startup warning is logged — fine for a trusted LAN, set the hash for any network-reachable deployment.
- `POST /api/auth/login` verifies the password and starts a tower-sessions session (cookie); `GET /api/auth/me` reports `{ authenticated, auth_required }`; `POST /api/auth/logout` flushes the session.
- `RequireAuth` extractor guards all business `/api` routes (401 when a hash is set and the session isn't authed; passes through in open mode).
- Frontend: routes under `frontend/src/routes/(app)/` are guarded by `(app)/+layout.ts` (redirects to `/login?next=…` when `auth_required && !authenticated`); `/login` is public.

### RPC → REST

Every former Tauri command maps to one route: reads `GET`, creates `POST` (→ `201` + id), updates `PUT` (→ `204`), deletes `DELETE` (→ `204`). `id` in the path, payloads in the JSON body. **Responses return the raw value (no `{data}` wrapper)** so the frontend wrapper return types match the old `invoke()` shapes.

### Frontend (SvelteKit)

- `frontend/` — the SvelteKit app (`package.json`, `vite.config.ts`, `svelte.config.js`, `src/`, `static/`). Builds to `frontend/build/`.
- `src/routes/(app)/` — Authenticated page components (file-based routing) behind the layout guard; `src/routes/login/` is the public login page.
- `src/lib/components/ui/` — Reusable UI components (Button, Input, Select, Dialog, etc.)
- `src/lib/components/layout/` — Layout components (Sidebar, PageHeader)
- `src/lib/api/` — Thin wrappers over the shared `request<T>(method, path, body?)` fetch helper in `client.ts` (sends cookies via `credentials: 'include'`, parses the `{error}` envelope, fires an unauthorized handler on 401). **`request()` replaces the old `invoke()`** — the wrapper signatures are otherwise unchanged.
- `src/lib/types/index.ts` — TypeScript interfaces for all entities
- Vite dev server proxies `/api` → `http://localhost:3002`; in production axum serves both the embedded SPA and the API.

### Database

- SQLite via SeaORM — all queries go through typed Rust entities
- Migrations run automatically via `Migrator::up()` at server startup (`db.rs::init`)
- `DATABASE_URL` selects the DB (dev default `sqlite:./kammerz.db?mode=rwc`). Carry over an existing Mac catalog by copying it to the configured path before first run — `seaql_migrations` is already populated so `Migrator::up` is a no-op.
- Single-connection pool (`max=min=1`) so the OFF→migrate→ON pragma sequence is deterministic and an in-memory test DB stays alive for the life of the pool. A single-user catalog never needs concurrent writers.
- SQLite pragmas: `journal_mode=WAL`, configurable `busy_timeout` (`SQLITE_BUSY_TIMEOUT_MS`, default 5000), `foreign_keys`. **Critical**: SQLx defaults `PRAGMA foreign_keys=ON` on SQLite connections — `db.rs` explicitly sets `foreign_keys=OFF` before `Migrator::up()` and re-enables `ON` after. Table-rebuild migrations (CREATE new → INSERT → DROP old → RENAME) trigger SQLite's implicit DELETE on DROP TABLE, which cascades through `ON DELETE CASCADE` (deleting junction rows) and `ON DELETE SET NULL` (NULLing FK columns).
- Junction table gotcha: Entity file is `camera_lens.rs` but the SQLite table name is `camera_lenses` (plural). Always check `#[sea_orm(table_name = "...")]` in entity files — don't guess from the filename.
- The only schema addition vs. a pre-port Tauri DB is the `tower_sessions` table (created by the session store's own migration). Core catalog schema is identical.

## Important Conventions

### UX Rules
- **Always confirm destructive actions.** Never delete data without user confirmation.
- Back navigation: Detail pages use `PageHeader`'s `backHref`/`backLabel` props for consistent back links. Cross-entity links (e.g., developments→roll, dashboard→roll, search→roll/camera) pass `?from=<source>` query param; detail pages read this via `$page.url.searchParams.get('from')` and map it to the correct back route. See `backRoutes` map in `rolls/[id]/+page.svelte`.
- Owned/Sold filtering: List pages with `date_sold` fields (cameras, lenses) use client-side All/Owned/Sold tab buttons with a `$derived()` filter. No backend changes needed to add this to a new list page.
- Fixed-lens cameras: Structural invariant — camera creation with "Fixed Lens" mount MUST always call `createCameraWithLens()` (never plain `createCamera()`). Show read-only lens indicators everywhere — lens list cards ("Fixed on [Camera]"), lens edit dialog (accent banner), roll default lens (locked text), shot lens dropdown (read-only), Quick Entry (locked text). Camera detail page shows "Built-in Lens" section (no unlink/link/default-change controls). Camera edit locks the mount field to read-only. Detect via mount name: `lensMounts.find(m => m.id === mountId)?.name === 'Fixed Lens'` or `lensMounts.some(m => m.id === mountId && m.name === 'Fixed Lens')` — never hardcode mount IDs.
- Shot lens defaults: Smart cascade — fixed lens (auto-locked) > last-used lens on roll > `roll.lens_id` (roll default) > `camera.default_lens_id` (camera default) > empty.
- Shot date defaults: Smart cascade — last shot's date on roll > `roll.date_loaded` (first shot) > empty. Date persists as a session default across "Save & Next".
- Development auto-prompt: Moving status to "at-lab" auto-opens lab dev dialog; "developing" auto-opens self dev dialog (only if neither dev record exists). Lab and self dev are mutually exclusive — UI hides "+ Lab" / "+ Self" buttons once one exists.
- Data-driven status sync: Roll status auto-advances and auto-reverts based on related data, handled transactionally in backend commands. Two backend helpers in `roll_service.rs`: `auto_sync_status()` is a conditional *set* (if status ∈ from-set, set to target) used for the reverts; `sync_lab_dev_status()` / `sync_self_dev_status()` (built on `advance_status_along()`) *advance forward* along the lab/self flow from any earlier rung, including a roll orphaned mid-path. Rules: first shot added → `loaded→shooting`; **lab dev created → `→lab-done` if a `date_received` is recorded, else `→at-lab`**; **self dev created → `→developed` if a `date_processed` is recorded, else `→developing`** (the dev record's date fields are the status signal — kammerz-afc); all shots deleted → `shooting/shot→loaded`; lab dev deleted → `at-lab/lab-done→shot`; self dev deleted → `developing/developed→shot`. The create-advance is forward-only: a roll already past the target on its flow (e.g. `scanned`) is untouched, and a roll orphaned at `at-lab`/`developing` advances to `lab-done`/`developed` in one action when the completing date is supplied. Status beyond a data type's range is not affected (e.g., deleting a dev record at `scanned` stays at `scanned`). Manual backward moves (chevron clicks) still require `ConfirmDialog` — the confirmation exists for when the user intentionally contradicts the data state. Never use frontend `$effect` for status sync — the backend owns this logic.
- Shot dialog "Save & Next": Keeps dialog open after save, resets per-shot fields (aperture, shutter, notes), preserves session defaults (date, location, lens), auto-suggests next frame number. Only shown in add mode (not edit).
- Dashboard roll sections: "In the Field" shows `loaded`/`shooting` rolls (in a camera). "In the Darkroom" shows `shot`/`at-lab`/`lab-done`/`developing`/`developed`/`scanned` rolls (post-shooting pipeline, sorted by status progression). "Needs Attention" shows rolls with `!camera_id` (excluding archived). All non-archived rolls must appear in at least one clickable section.

### Svelte 5 Patterns
- Use `$state()`, `$derived()`, `$effect()`, `$props()`, `$bindable()` — no legacy `let` reactivity.
- Use `onclick={handler}` on buttons instead of `<form onsubmit>`. (Historically a Tauri WebKit workaround; kept as a project convention for consistency across the existing pages.)
- Button component passes `onclick` via `{...rest}` spread to the native `<button>` element.
- Detail page edit mode: When a page has view/edit toggle, maintain parallel `$derived` vars — e.g., `selectedCamera` (from saved `roll.camera_id`) for shot defaults vs `editSelectedCamera` (from `editCameraId` form state) for edit-mode film stock/lens filtering.

### axum / SeaORM Patterns
- Handlers take `RequireAuth` first (enforces the session guard), then `State<DatabaseConnection>` (or `State<AppState>`/`State<AppConfig>` when config is needed), then `Path`/`Query`/`Json` extractors; they delegate to services and return `AppResult<Json<T>>` / `StatusCode::NO_CONTENT` / `(StatusCode::CREATED, Json(id))`.
- Services are static async methods on unit structs (e.g., `CameraService::list_all(&db)`)
- Entities use `String` for timestamps (SQLite TEXT), `Option<T>` for nullable fields
- For joined queries (e.g., rolls with camera/film stock), use `#[derive(FromQueryResult)]` with raw SQL
- For junction table data (e.g., shot↔lens), prefer batch queries per parent (e.g., `get_lenses_for_roll_shots(roll_id)`) over per-row queries. Avoids N+1 query overhead.
- DTOs in route modules handle create/update payloads; services work with `ActiveModel` directly
- Transaction trait imports: When using SeaORM entity operations directly inside a `transaction` closure (instead of via service methods), additional traits must be in scope: `ActiveModelTrait` (for `.insert()`, `.update()` on models), `ColumnTrait` + `QueryFilter` (for `.filter(...eq(...))` queries), `PaginatorTrait` (for `.count()`). The `create_shot` and `delete_shot` handlers show the full import set.
- Changes to backend files (`src/`, `entity/`, `migration/`, `Cargo.toml`) require `cargo` to recompile (`just dev-backend` or `cargo run`)
- Migration raw SQL gotcha: `execute_unprepared()` auto-commits each statement. If a migration fails midway, partial data persists but the migration isn't recorded in `seaql_migrations` — so it re-runs on next start, creating duplicates. Use `INSERT OR IGNORE` for data, `IF NOT EXISTS` for `CREATE INDEX`/`CREATE TABLE`, and `DROP ... IF EXISTS` before recreating objects. For tables without unique constraints (cameras, lenses), failures require manual cleanup via `sqlite3`.
- Seed migration pattern: Use subqueries for FK resolution (`(SELECT id FROM lens_mounts WHERE name = 'Nikon F')`) instead of hardcoded IDs — IDs vary across environments and are fragile across migration reorders.
- Fixed-lens seed pattern: Each fixed-lens camera requires 4 SQL statements in order: INSERT camera → INSERT lens → INSERT camera_lenses junction → UPDATE camera.default_lens_id. See migration 013 for the template.
- Table-rebuild migration safety: Any migration that uses DROP TABLE on a parent table (cameras, lenses, etc.) will cascade-delete rows in child tables (camera_lenses, shot_lenses) if FK enforcement is on. The `db.rs` fix (`PRAGMA foreign_keys=OFF` before migrations) prevents this, but if writing new table-rebuild migrations, always verify the FK pragma state. Migration 020 is a repair migration for data lost before the pragma fix was in place.
- Batch child merge pattern: When a list endpoint needs parent rows + child collections (e.g., developments + stages), fetch parents first, collect IDs, batch-fetch children via `IN (...)`, merge via `HashMap<parent_id, Vec<Child>>` with `.remove()` (not `.get()`) to avoid cloning. See `list_all_self_developments` in `development_service`.
- DeriveActiveEnum pattern: Columns with constrained string values (status, format, type) use `#[derive(DeriveActiveEnum)]` with `#[sea_orm(string_value = "...")]` + `#[serde(rename = "...")]`. Enums defined in entity files alongside the Model (e.g., `RollStatus` in `roll.rs`). Raw SQL `FromQueryResult` structs can use these enum types directly — SeaORM's `TryGetable` handles deserialization. When adding new enum variants, update both the Rust enum and the TypeScript union type in `src/lib/types/index.ts`.
- Error helper: `routes/mod.rs::friendly_err(context, error)` maps DB constraint errors to user-friendly messages, returned as `AppError::UnprocessableEntity`. `context` is a noun ("roll", "camera", "film stock"). Uses `raw.find()` (not `strip_prefix`) for constraint detection because SeaORM wraps errors with context prefixes. Apply to all create/update/delete `map_err` closures — not read-only queries (reads never trigger constraint errors).
- Composite routes: When a detail page needs data from multiple tables, use a single route that aggregates all queries (e.g., `GET /api/rolls/{id}/detail` returns roll + shots + shot_lenses + devs + stages). Reduces HTTP round-trips.
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
- Frontend `request()` calls (and the `src/lib/api/` wrappers over them) reject with `ApiRequestError` on a non-2xx response, carrying the backend `{error: {code, message}}`. Wrap in try/catch with user-visible error display. A 401 fires the registered unauthorized handler (redirect to `/login`).
- Always validate required fields client-side before API calls (brand, model, mount, etc.). Show inline `error` state text — don't rely on backend DB constraint errors which are opaque to users.

### UI Design
- Follow the design system in `UI_DESIGN.md` — colors, typography, component styling, layout patterns, and design principles.
- All colors use CSS custom properties defined in `frontend/src/app.css` via Tailwind's `@theme`. Never use raw hex colors.
- Fonts: DM Sans (UI), IBM Plex Mono (data), Instrument Serif (display). Self-hosted in `frontend/static/fonts/`.
- Keep `UI_DESIGN.md` updated when design decisions change.

### Git / Branch Hygiene
- **PRs are squash-merged.** `git branch --merged origin/main` therefore *lies* — squash breaks ancestry, so merged branches show as unmerged. Verify with `gh pr list --state merged --json headRefName` before deleting. GitHub doesn't auto-delete branches on merge, so remote branches accumulate; prune with `git push origin --delete <branch>`.

## Reference

- Another SeaORM + SQLite project by the same author: `~/Development/projects/financier` (same SeaORM patterns). The axum + tower-sessions + rust-embed server structure mirrors `~/Development/projects/chorez`.
- `UI_DESIGN.md` documents the visual design system (colors, typography, components, layout)


<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:ca08a54f -->
## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd dolt push
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
<!-- END BEADS INTEGRATION -->
