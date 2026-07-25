# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0/).

## [Unreleased]

## [0.1.0] - 2026-07-25

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
- Single shared-password authentication (argon2 + SQLite-backed sessions) with a
  secure-by-default posture: the server binds only the loopback interface in open
  (no-password) mode, and the billable AI-import endpoints are rate-limited. An
  open LAN-trust mode is available for fully trusted networks.
- Single-binary deployment: the SvelteKit build is embedded via rust-embed and
  served alongside the API — no separate web server or Node runtime. Includes a
  cross-compiled systemd deploy flow and WAL-safe backups (a one-tap
  `/api/backup` snapshot plus documented `sqlite3` and cold-copy methods).
- AGPL-3.0 license.

[unreleased]: https://github.com/turboladen/kammerz/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/turboladen/kammerz/releases/tag/v0.1.0
