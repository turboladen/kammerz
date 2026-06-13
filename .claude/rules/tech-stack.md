# Tech Stack

- **axum 0.8** (Rust) — HTTP server exposing `/api/*` JSON endpoints and serving the embedded SvelteKit build
- **SvelteKit** with **Svelte 5** runes (`$state`, `$derived`, `$effect`, `$props`, `$bindable`)
- **Bun** as package manager and JS runtime
- **SQLite** via **SeaORM 1.1** (Rust ORM) — typed entities, services, and migrations
- **tower-sessions** (SQLite-backed) + **argon2** — single-password session auth
- **rust-embed** — the SvelteKit `frontend/build` is baked into the release binary (SPA fallback in `main.rs`)
- **Tailwind CSS 4** with `@tailwindcss/vite` plugin and custom dark theme via `@theme`
- **adapter-static** for SvelteKit (SPA — `ssr = false`, served as static assets by axum)
- **Rust edition 2024**, workspace MSRV `rust-version = "1.85.0"` (root crate). `std::sync::LazyLock` is available (no `once_cell`). Gotcha: `std::env::set_var`/`remove_var` are `unsafe` in edition 2024 (env mutation races concurrent readers) — in parallel tests gate the write behind a `std::sync::Once` for a real happens-before argument, not a "same value" hand-wave (kammerz-egu).
