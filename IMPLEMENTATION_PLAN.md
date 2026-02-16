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

## Phase 3: Shot Entry & Quick Entry 🔲

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

## Phase 4: Development Tracking 🔲

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

## Phase 5: Search, Import & Polish 🔲

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

All 12 database tables are created and migrated (Phase 1). No further migrations are needed for Phases 3–4 — the schema already covers shots, shot-lenses, development (lab and self), and dev stages.

| Table | Phase Built | UI Built |
|---|---|---|
| `cameras` | 1 | ✅ |
| `camera_maintenance` | 1 | ✅ |
| `lenses` | 1 | ✅ |
| `camera_lenses` | 1 | 🔲 Phase 3 |
| `film_stocks` | 1 | ✅ |
| `labs` | 1 | ✅ |
| `rolls` | 1 | ✅ |
| `shots` | 1 | 🔲 Phase 3 |
| `shot_lenses` | 1 | 🔲 Phase 3 |
| `development_lab` | 1 | 🔲 Phase 4 |
| `development_self` | 1 | 🔲 Phase 4 |
| `dev_stages` | 1 | 🔲 Phase 4 |
