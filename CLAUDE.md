# Kamerz

Film photography catalog desktop app built with Tauri 2 + SvelteKit + SQLite.

## Tech Stack

- **Tauri 2** (v2.10.0) — native macOS app using system WebKit webview
- **SvelteKit** with **Svelte 5** runes (`$state`, `$derived`, `$effect`, `$props`, `$bindable`)
- **Bun** as package manager and JS runtime
- **SQLite** via `tauri-plugin-sql` (v2.3.2) with migration runner
- **Tailwind CSS 4** with `@tailwindcss/vite` plugin and custom dark theme via `@theme`
- **adapter-static** for SvelteKit (Tauri has no server — serves static files)

## Commands

- `bun run tauri dev` — Run the app in development mode (Tauri + Vite)
- `bun run tauri build` — Build the production .app bundle
- `bun run build` — Build SvelteKit frontend only (useful for quick compile checks)
- `bun run dev` — Run Vite dev server only (no Tauri backend — DB calls will fail)

## Architecture

### Frontend (SvelteKit)

- `src/routes/` — Page components (SvelteKit file-based routing)
- `src/lib/components/ui/` — Reusable UI components (Button, Input, Select, Dialog, etc.)
- `src/lib/components/layout/` — Layout components (Sidebar, PageHeader)
- `src/lib/db/` — Database access layer (thin wrappers around `tauri-plugin-sql` calls)
- `src/lib/types/index.ts` — TypeScript interfaces for all entities
- `ssr = false` and `prerender = false` in root layout (required for Tauri)

### Backend (Rust/Tauri)

- `src-tauri/src/lib.rs` — Tauri app setup, registers SQL plugin with migrations
- `src-tauri/migrations/` — SQLite schema and seed data (embedded at compile time via `include_str!`)
- `src-tauri/capabilities/default.json` — Tauri 2 permission grants

### Database

- SQLite via `tauri-plugin-sql` — JS calls `db.select()` / `db.execute()` directly
- Migrations run automatically on app start via tauri-plugin-sql's migration runner
- Parameter binding uses `$1, $2, ...` style (not `?`)

## Important Conventions

### UX Rules
- **Always confirm destructive actions.** Never delete data without user confirmation.
- Back navigation: Detail pages use `PageHeader`'s `backHref`/`backLabel` props for consistent back links.

### Svelte 5 Patterns
- Use `$state()`, `$derived()`, `$effect()`, `$props()`, `$bindable()` — no legacy `let` reactivity.
- Use `onclick={handler}` on buttons instead of `<form onsubmit>`. Form submission events don't work reliably in Tauri's WebKit webview inside conditional Svelte blocks.
- Button component passes `onclick` via `{...rest}` spread to the native `<button>` element.

### Tauri 2 Gotchas
- **SQL plugin permissions**: `sql:default` only grants `select` and `load`. Write operations need `sql:allow-execute` in `src-tauri/capabilities/default.json`.
- Changes to `src-tauri/` files (Rust, capabilities, Cargo.toml) require Tauri to recompile. The dev server usually auto-restarts, but may need manual restart.
- The `beforeDevCommand` and `beforeBuildCommand` in `tauri.conf.json` run the frontend build automatically.

### Camera Format Dropdown
- Includes generic "Medium Format" and "Large Format" options for cameras that support multiple backs (e.g., Mamiya RB67).
- Format labels use "Medium Format: 6x6" style (not "6x6 (Medium Format)").

### Error Handling
- Wrap all `db.execute()` calls in try/catch with user-visible error display. Without this, permission or SQL errors fail silently (unhandled promise rejection).

## Reference

- Another Tauri 2 + SQLite project by the same author: `~/Development/projects/financier` (uses SeaORM + Tauri commands instead of tauri-plugin-sql)
