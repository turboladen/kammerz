# Kamerz

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
- SQLite pragmas: `journal_mode=WAL`, `busy_timeout=5000`

## Important Conventions

### UX Rules
- **Always confirm destructive actions.** Never delete data without user confirmation.
- Back navigation: Detail pages use `PageHeader`'s `backHref`/`backLabel` props for consistent back links.

### Svelte 5 Patterns
- Use `$state()`, `$derived()`, `$effect()`, `$props()`, `$bindable()` — no legacy `let` reactivity.
- Use `onclick={handler}` on buttons instead of `<form onsubmit>`. Form submission events don't work reliably in Tauri's WebKit webview inside conditional Svelte blocks.
- Button component passes `onclick` via `{...rest}` spread to the native `<button>` element.

### Tauri 2 / SeaORM Patterns
- Commands receive `State<'_, AppState>`, delegate to services, return `Result<T, String>`
- Services are static async methods on unit structs (e.g., `CameraService::list_all(&db)`)
- Entities use `String` for timestamps (SQLite TEXT), `Option<T>` for nullable fields
- For joined queries (e.g., rolls with camera/film stock), use `#[derive(FromQueryResult)]` with raw SQL
- DTOs in command files handle create/update payloads; services work with `ActiveModel` directly
- Changes to `src-tauri/` files (Rust, capabilities, Cargo.toml) require Tauri to recompile

### Camera Format Dropdown
- Includes generic "Medium Format" and "Large Format" options for cameras that support multiple backs (e.g., Mamiya RB67).
- Format labels use "Medium Format: 6x6" style (not "6x6 (Medium Format)").
- Camera format → film stock format mapping: `35mm`→`135`, all medium format variants (`6x4.5`–`6x9`)→`120`, large format sizes map directly (`4x5`→`4x5`, etc.). Don't filter out non-matching formats — only reorder (cameras can use different backs).

### Component Patterns
- `ComboInput` dropdown options use `onmousedown` (not `onclick`) to beat the blur/click race condition.
- `Select` options support an optional `disabled` property (used for visual dividers like `── Other formats ──`).
- Use `$derived.by(() => { ... })` when derived state needs multi-line logic; `$derived(expr)` for one-liners.

### Error Handling
- Frontend `invoke()` calls return promises that reject on error. Wrap in try/catch with user-visible error display.

## Reference

- Another Tauri 2 + SQLite project by the same author: `~/Development/projects/financier` (same SeaORM patterns)
- `IMPLEMENTATION_PLAN.md` tracks phase-by-phase development progress
