# Design: Port Kammerz from Tauri desktop → axum + Svelte web app

> **Status:** Implemented — dated design record, kept as history. Current architecture decisions live in the [ADR index](../../adr/README.md).

- **Date:** 2026-05-29
- **Status:** Approved (direction); pending implementation plan
- **Author:** Steve Loveless (with Claude)

## Decision

Port Kammerz from a Tauri 2 desktop app to a self-hosted **axum + SvelteKit web
app**, following the established chorez/financier pattern, deployed as a systemd
unit on the existing home NAS.

### Why

For this specific app the usual desktop→web migration cost barely applies, while
the upside lands directly on stated needs:

- **Shared core.** Both stacks already use SeaORM + SQLite with
  migrations-on-startup. Entities, services, and the migration crate move over
  nearly untouched.
- **Clean transport seam.** Every `invoke()` call lives in the 12 files under
  `src/lib/api/` and nowhere else in the frontend. Components and routes import
  typed functions (`listCameras()`), so the frontend port is confined to
  rewriting those 12 wrappers to use `fetch`.
- **No binary assets.** Kammerz is a _metadata_ catalog (rolls, shots, gear, dev
  recipes). It stores no scans or images, removing the hardest part of porting a
  desktop app to web (upload/blob storage, asset serving).
- **Frontend already web-shaped.** SvelteKit is on `adapter-static` with
  `ssr=false` / `prerender=false` — exactly what `rust-embed` SPA serving
  expects.
- **Stated wants.** Multi-device LAN access (laptop + phone) now; field access
  via the home VPN; consolidation onto existing self-host infra; escape from
  documented macOS WebKit quirks.

The one genuinely new surface area is auth/exposure, which is already solved in
the author's other apps.

## Target architecture

### Backend (axum)

- axum HTTP server; SvelteKit `build/` embedded via `rust-embed` with an SPA
  fallback to `index.html`.
- SeaORM + SQLite reused as-is; `Migrator::up()` runs on startup (unchanged
  behavior). DB path from `DATABASE_URL`, defaulting under the systemd data dir.
- REST `/api/*` JSON endpoints replace the Tauri `generate_handler!` command
  registry — one handler per current command. Existing command DTOs become
  request/response bodies.

### Auth

- **Single shared password.** Argon2 hash supplied via env / `.env` at deploy
  time.
- `tower-sessions` with a SQLite-backed store; cookie is `HttpOnly`,
  `SameSite=Lax`, and `Secure` when behind TLS/VPN.
- Middleware guards `/api/*` and the app shell. `/login` is the only
  unauthenticated route. Mirror chorez's tested session middleware.

### Frontend (SvelteKit)

- Stays on `adapter-static`, `ssr=false`.
- Rewrite the **12 `src/lib/api/*` wrappers** from `invoke(name, args)` to a
  typed `fetch` client modeled on chorez's `api.ts`: sends credentials, handles
  JSON + structured errors, redirects to `/login` on 401.
- Components, routes, and `types/index.ts` are unchanged.

### AI import

- `import_service.rs` keeps calling the Anthropic API via `reqwest`, now
  server-side. The API key moves to server config (an improvement: no key shipped
  in a desktop bundle, no CORS).

### Deployment

- systemd unit on the home NAS, same pattern as chorez/financier: unprivileged
  user, data dir, `.env` for `DATABASE_URL` / `PORT` / session secret / password
  hash / Anthropic key, restart-on-failure.
- Binds to the LAN. **L2 (home VPN)** provides field access at the network layer
  — no public port, no manual TLS-cert management, and nothing installed on the
  NAS or the app. See L2 below.

## What moves vs. what is new

| Category            | Items                                                                                                                                                        |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Reuse ~verbatim** | All 14 entities, all services, the migration crate, all routes/components, `src/lib/types/index.ts`                                                          |
| **Rewrite**         | Command layer → axum handlers; 12 api wrappers → fetch client; new `main.rs` (router, app state, rust-embed, sessions, auth middleware); config/env handling |
| **Remove**          | Tauri deps, `lib.rs` builder / AppState / invoke registry, `capabilities/`, `@tauri-apps/*` JS deps, `tauri.conf.json`                                       |

## Access staging

- **L1 — LAN web (in scope).** Laptop + phone on the home LAN hit the same
  catalog with no sync. This is the deliverable of this spec.
- **L2 — Remote access via the home VPN (in scope, ops-only).** Use the UniFi
  gateway's built-in VPN (Teleport / WireGuard VPN server) so the phone joins the
  home LAN remotely and reaches the app at its LAN address in the field. Purely
  network-layer: no code, nothing installed on the NAS or the app. Planning
  deferred; the `../unifi-management` Claude+UniFi MCP server is the tool for
  configuring it when we get there.
- **L3 — Offline PWA (out of scope / future).** Local queue + sync for no-signal
  jotting. Deferred until the field habit proves it is needed.

## Verification

The current CLAUDE.md "no preview tools" rule exists because Tauri's `invoke()`
needs the native WebKit IPC bridge. As a plain browser app served by axum,
**normal browser / Playwright verification becomes available** — a net
improvement to the dev/verify loop. CLAUDE.md should be updated to reflect this
once the port lands.

## Out of scope

- Offline PWA (L3).
- Multi-user accounts / roles (single shared password only).
- Public-internet exposure beyond the home VPN.
- Image/scan storage (remains a metadata-only catalog).
- **Mobile/field UX work — split into its own follow-up spec.** That includes the
  responsive pass and a phone-optimized, low-overhead quick-entry flow (seeded by
  the existing `quick-entry` route). This spec delivers a functional web app
  reachable from a phone; the dedicated field-jotting experience is separate
  work that depends on this port landing first.

## Risks & mitigations

- **Feature-parity regression window** across ~11 routes → shared core limits
  surface; build a parity checklist (route-by-route) as part of the plan.
- **Auth/exposure correctness** (must not serve the app shell unguarded) →
  reuse chorez's tested session middleware; add a test asserting unauthenticated
  requests are rejected.
- **SQLite concurrency** → non-issue at solo / few-device scale with WAL already
  enabled.
- **Scope creep into mobile UX** → explicitly deferred to the follow-up spec.

## Follow-up specs

1. **Mobile/field UX** — responsive shell + thumb-optimized quick-entry; depends
   on this port.
2. _(Possible later)_ **L3 offline PWA** — only if field usage demonstrates the
   need.
