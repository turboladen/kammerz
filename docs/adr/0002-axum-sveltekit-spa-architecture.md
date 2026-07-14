# ADR-0002: axum + embedded SvelteKit SPA in a single self-hosted binary

- **Status:** Accepted
- **Date:** 2026-05-29
- **Related:** `.claude/rules/tech-stack.md`, `.claude/rules/architecture.md`, `src/main.rs` (rust-embed SPA fallback), `src/db.rs` (single-connection pool + pragma sequence)

## Context

Kammerz began as a Tauri desktop app. The goal shifted to a **self-hosted web app**
reachable from any device on a home LAN (and over the gateway VPN from the field),
backed by a single-user SQLite catalog. We needed a deployment shape that a solo
maintainer can build, ship, and run on a small NAS/home-server box without a
multi-service stack, while reusing the existing Svelte UI and SQLite data model.

## Decision

Serve everything from **one Rust binary**:

- **axum** exposes `/api/*` JSON endpoints and serves the frontend.
- The **SvelteKit** app builds as a static SPA (`adapter-static`, `ssr = false`)
  and is **embedded into the binary** via `rust-embed`; axum serves it with an SPA
  fallback. No separate web server, no Node runtime in production.
- **SQLite via SeaORM** for typed entities/migrations; a single-connection pool
  (a single-user catalog never needs concurrent writers).
- **tower-sessions (SQLite-backed) + argon2** single shared-password auth; open
  (LAN-trust) mode when no password hash is set.
- Ship as a cross-compiled binary deployed over SSH to the box; no container, no
  GitHub releases.

## Consequences

- **Positive:** one artifact to build and deploy; the frontend and API are always
  version-matched; trivial to run on a low-powered box. Dev keeps the fast Vite
  loop (`:5273` proxying `/api` → axum `:3002`).
- **Positive:** SQLite + single-connection pool makes the DB lifecycle
  deterministic (the FK-OFF → migrate → FK-ON sequence, in-memory test DBs).
- **Negative / limits:** single-writer, single-user by design — not built for
  concurrent multi-user access. SPA-only means no SSR/SEO (irrelevant for a private
  catalog). Scaling beyond one host would require revisiting this ADR.
- Downstream conventions (embedded build wiping `frontend/build`, the pragma
  sequence, session table) follow from this shape and are documented in
  `.claude/rules/`.
