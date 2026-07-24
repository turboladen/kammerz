# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0/).

## [Unreleased]

## [0.1.0] - TBD

First release of Kammerz — a self-hosted film photography catalog. A single Rust
(axum) binary serves a SvelteKit single-page app and a JSON API over SQLite,
designed to run on a home server / NAS and reach the field over a gateway VPN.

### Added

- Catalog for the full analog kit: cameras, lenses, lens mounts, film stocks,
  and labs, with owned/sold tracking, fixed-lens cameras, mount-compatibility
  sorting, and automatic disambiguation of duplicate gear.
- Rolls and per-shot logging — aperture, shutter, lens, date, time-of-day,
  location, and notes — with Quick Entry, a wrap-to-width film strip, a shots
  table view, and prev/next navigation in the shot editor.
- Activity-based roll lifecycle: shooting, development, scanning,
  post-processing, and archiving phases derive from recorded dates and dev
  records, driving the roll activity board, dashboard sections, and pipeline bar.
- Development records for lab and self-developing, including a canonical
  chemistry reference with self-learning autocomplete.
- AI-assisted import that parses roll data via the Anthropic API.
- Search, statistics, and a dashboard that surfaces rolls needing attention
  (e.g. negatives to collect).
- Single shared-password authentication (argon2 + SQLite-backed sessions), with
  an open LAN-trust mode for fully trusted networks.
- Single-binary deployment: the SvelteKit build is embedded via rust-embed and
  served alongside the API — no separate web server or Node runtime. Includes a
  cross-compiled systemd deploy flow and WAL-safe backups (a one-tap
  `/api/backup` snapshot plus documented `sqlite3` and cold-copy methods).
- AGPL-3.0 license.

### Security

- Open (no-password) mode now binds the loopback interface by default instead of
  all interfaces, so an unauthenticated instance is not inadvertently exposed
  across the LAN.
- Rate-limited the billable Anthropic-backed import endpoints.
- Allowlisted and trimmed settings keys and rejected a whitespace-only API key,
  so only known settings can be written.
- Hardened error responses to stop leaking raw database error text.
- Shortened the session inactivity window.

### Fixed

- Imported shots no longer store `f/`-prefixed aperture/shutter values, which had
  rendered doubled (e.g. `f/f/5.6`).
- Added in-flight guards to dialog Save/Add buttons and pre-submit validation
  (lens mount on camera edit, shot time) so a double-click or invalid field can
  no longer create bad or duplicate records.
- `friendly_err` now maps not-found and business-rule errors correctly on
  transactional updates (404 instead of a misleading 500).
- The `/api/health` check no longer stalls behind an in-progress backup.
- A handler panic now returns the standard `{error}` JSON envelope instead of a
  bare connection drop.
- Search minimum-length now counts characters rather than bytes.
- Made the m019 schema-hardening migration and the 013–016 seed migrations
  re-run-safe, so a crash mid-migration no longer wedges startup or duplicates
  seeded gear, and fixed `datetime('now')` column defaults.

### Changed

- Corrected and consolidated the deployment documentation (bootstrap flow,
  cross-compile linker setup, backup/restore guidance).
- Made transactional and non-transactional error handling consistent and removed
  dead code paths.
- Expanded automated test coverage across the core daily loop, auth session
  round-trip, API client, lifecycle derivation, and migration backfills.

[unreleased]: https://github.com/turboladen/kammerz/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/turboladen/kammerz/releases/tag/v0.1.0
