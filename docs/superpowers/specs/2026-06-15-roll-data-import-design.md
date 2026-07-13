# Roll Data Import — Design

**Date:** 2026-06-15
**Status:** Approved design, pending spec review
**Author:** Steve Loveless (with Claude)

> **Historical note (preserved into `main` after the fact, 2026-07):** This design
> predates the removal of `date_fuzzy` (epic `kammerz-qdk`). Where it describes
> parsing/emitting fuzzy dates, the current convention is a concrete best-guess date
> plus a notes annotation — reconciliation is tracked by `kammerz-31c` / `kammerz-btv`.
> Kept for the historical record of the one-time 300+ roll import (now complete).

## Problem

~300+ rolls of historical film-photography data live outside Kammerz, spread across
three sources with **varying levels of detail**:

- **Apple Notes** — the **source of truth**. 300+ rolls (freeform / inconsistent
  detail) plus a **half-finished camera/lens catalog**. These are the originals.
- **Apple Numbers** — ~36 rolls, the most _structured_ (tabular).
- **NotePlan** — a **partial, abandoned migration** of the Apple Notes rolls (the user
  started moving rolls over a couple years ago, never settled on a system, stopped). Almost
  certainly a subset/duplicate of the Notes originals with no added detail — **likely
  skipped entirely** (see Phase 3).

The goal is to get this data into Kammerz **accurately**, with the **least risk**.
Raw SQL / DB-level import is rejected as too error-prone. We want every write to go
through the same validated REST endpoints the app itself uses.

### A gift in the data: roll IDs encode the camera

Roll IDs follow a prefix scheme where the prefix identifies the camera — e.g.
`LR6-{1,2,3,…}` are Leica R6 rolls, `LSL-{1,2,3,…}` are Leica SL rolls. Kammerz's import
already matches on exactly this (`camera_prefix_guess` → `camera.prefix`). So camera
reconciliation is driven by **enumerating the distinct prefixes and mapping each to a
camera**, rather than parsing camera names out of freeform prose — far more reliable.

## Key decisions (settled during brainstorming)

1. **No Kammerz MCP server.** Deferred as YAGNI. The import is _better_ served without
   one (Claude is already the parser, and the write endpoints already exist). The
   lasting "talk to your catalog" use cases are mostly occasional and the web UI beats
   chat for most of them. An MCP server is purely additive and wraps a REST API that
   already exists, so it can be added later on its own merits if the user misses it —
   nothing here depends on it.
2. **One-time, agent-driven batch import.** Pipeline:
   `bulk export → local corpus → Claude (parse + match) → existing REST endpoints (write) → human review`.
   Apple Notes is exported in one pass via **AppleScript (`osascript`), run by the user**
   (the official automation API — not file-rummaging; the user keeps control of the read),
   producing a stable local corpus file Claude re-parses across sessions without
   re-touching Notes. Numbers is exported to CSV. NotePlan's MCP server is **not needed**
   unless the spot-check (Phase 3) finds enriched rolls there.
3. **Reuse existing endpoints; build nothing.** `POST /api/import/roll` writes a roll +
   its shots transactionally. Vocabulary is created via `/api/cameras`,
   `/api/cameras/with-lens`, `/api/lenses`, `/api/lens-mounts`, `/api/film-stocks`,
   `/api/labs`. We **skip `/api/import/parse`** — Claude parses directly (no API-key
   cost, no round-trip).
4. **Write target: local dev `:3002`** (open auth in dev). This local DB becomes the one
   the user eventually deploys to the NAS. Because rolls reference catalog rows by
   integer ID, importing everything into one DB keeps IDs internally consistent.
5. **Catalog starts empty/seed.** Building the gear vocabulary (cameras, lenses, film
   stocks, labs, mounts) is _part of_ the import, not a precondition.
6. **Review mechanism: staging-doc review (Approach A)** for the freeform sources.
   Lighter direct-write-then-fix-in-app (Approach B) is acceptable for the structured
   Numbers export.
7. **Sequence: Numbers (36, structured) first as a pipeline dry-run**, then Apple Notes
   (300+). The Numbers pass validates the end-to-end flow on the easy data, seeds part of
   the vocabulary, and surfaces endpoint/validation surprises before the big batch.

## Architecture: two phases

Rolls reference cameras/lenses/film-stocks by **integer ID**, and `roll_id` carries a
UNIQUE index. Two consequences shape the whole design:

- **Reconcile the vocabulary before pouring in rolls.** Sweep every note for distinct
  gear mentions, dedup (notes will spell the same camera multiple ways), create each
  once, record a `name → id` lookup. Then rolls resolve cleanly. The **roll-ID prefix**
  (`LR6-…` = Leica R6, `LSL-…` = Leica SL) is the spine of camera reconciliation; the
  half-finished catalog note supplies the gear's real specs/serials.
- **The import is idempotent and resumable.** Re-running skips already-imported rolls
  because `roll_id` is UNIQUE — essential for a 300+-roll job spanning many sessions.

### Phase 0 — Setup, export & recon (small)

- **Apple Notes export:** Claude writes an AppleScript that dumps a chosen Notes folder —
  per note: title, folder, **creation + modification dates** (date-inference fallback when
  a note states no dates), and body — to a delimited local corpus file. **The user runs it**
  via the `!` prefix. Claude then reads the corpus, never Notes directly.
- **Numbers export:** user exports the rolls sheet to CSV.
- Confirm local dev Kammerz is up (`GET /api/health`), auth is open, and the catalog
  baseline is known (inspect seeded film stocks / lens mounts / cameras / lenses so
  matching has a baseline).
- Sample a handful of exported notes to calibrate the parser and confirm the catalog-note
  format before committing to a batch size.
- **Back up the dev DB file first** (`cp` the configured `DATABASE_URL` file) so the
  whole import is trivially revertible.

### Phase 1 — Vocabulary reconciliation

1. Seed from the **half-finished catalog note** (real gear specs/serials), then sweep
   **all** roll corpora for every distinct mention of: **lens mounts, cameras, lenses,
   film stocks, labs**. Enumerate the **distinct roll-ID prefixes** and map each to a
   camera (set `camera.prefix` so the existing prefix-match works downstream).
2. For each, match to an existing record or propose a new one with its fields.
   Dedup aggressively (`M67` ≡ `Mamiya RB67`).
3. Emit a **review artifact** (markdown tables: proposed mounts, cameras, lenses [with
   mount], film stocks, labs) for the user to approve / edit / merge.
4. On approval, create records via REST — **order matters for FKs**:
   mounts → film stocks / labs → lenses → cameras. Fixed-lens cameras (point-and-shoots
   etc.) go through `POST /api/cameras/with-lens` to honor the fixed-lens invariant;
   normal cameras through `POST /api/cameras` with `lens_mount_id`.
5. Record the resulting `name → id` lookup table (persisted as an artifact so Phase 2 —
   possibly a later session — can resolve against it).

### Phase 2 — Roll parsing + batched review

At 300+ rolls the **review pass is the dominant cost**, so lean on the structured signals
(roll-ID prefix → camera) to keep per-roll review light, and use **confidence-tiered
batches**: large batches (~20–25) for clean, prefix-resolved rolls; small (~8–10) for
messy/low-detail ones flagged during parsing. Per batch:

1. Parse each roll's freeform note into the `/api/import/roll` shape:
   `roll_id`, `status: archived` (old rolls are done), `frame_count`, dates
   (`date_loaded` / `date_finished`, with `date_fuzzy` for vague dates like
   "summer 2019"), `push_pull`, roll `notes`, and `shots[]`
   (`frame_number`, `aperture`, `shutter_speed`, `date`/`date_fuzzy`, `location`,
   `notes`, `lens_ids`). Resolve camera/film/lens to IDs via the Phase 1 lookup.
2. Emit a **per-batch staging artifact**: source note alongside the structured parse +
   resolved gear + **flags** (unmatched gear, fuzzy/ambiguous dates, duplicate or
   suspicious frame numbers — the endpoint rejects duplicate `frame_number` within a
   roll, so surface those _before_ writing).
3. User reviews and corrects inline; on approval, `POST /api/import/roll` per roll.
4. Track imported `roll_id`s (idempotency: UNIQUE index rejects re-imports; the tracker
   makes resume explicit). Use **beads** for cross-session progress tracking per project
   convention.

### Source sequencing & NotePlan disposition

The two-phase flow above runs per source, in this order:

1. **Numbers (~36, structured) — first, as the dry-run.** CSV columns map directly, so
   parsing is reliable and Approach B (write, then fix in-app) is acceptable. Validates the
   end-to-end flow and seeds part of the vocabulary before the big batch.
2. **Apple Notes (300+) — the main event.** Freeform; full Phase 1/2 with staging-doc
   review (Approach A). Direct extraction from the Notes SQLite blob store is intentionally
   avoided in favor of the AppleScript export (Phase 0).
3. **NotePlan — spot-check, then almost certainly skip.** Compare a few NotePlan rolls
   against their Apple Notes twins; if nothing was enriched during the abandoned migration,
   discard NotePlan entirely (no MCP server needed). Only if some rolls gained detail do we
   export NotePlan and run them through, relying on UNIQUE `roll_id` to skip the duplicates.

Across all sources, dedup against already-imported rolls is backstopped by the UNIQUE
`roll_id` index.

## Endpoints used (all existing)

| Purpose                      | Endpoint                      |
| ---------------------------- | ----------------------------- |
| Health / reachability        | `GET /api/health`             |
| Lens mounts                  | `GET/POST /api/lens-mounts`   |
| Film stocks                  | `GET/POST /api/film-stocks`   |
| Labs                         | `GET/POST /api/labs`          |
| Lenses                       | `GET/POST /api/lenses`        |
| Cameras (normal)             | `GET/POST /api/cameras`       |
| Cameras (fixed lens)         | `POST /api/cameras/with-lens` |
| Roll + shots (transactional) | `POST /api/import/roll`       |

No new server code. No `/api/import/parse` (Claude parses directly).

## Safety & verification

- **Local dev only**, open auth — no production data touched mid-import.
- **DB file backed up** before Phase 1.
- **All rolls land as `archived`** — historical, not active pipeline.
- **Human review at the parse step**, where freeform errors hide, before any write.
- **Idempotent** via UNIQUE `roll_id`; resumable across sessions.
- Post-batch verification: spot-check imported rolls in the app, or `GET` a sample back
  and diff against the source note.
- After import completes, promote the local DB to the NAS (the IDs travel with it).

## Open items (resolve during planning / execution)

- Which Apple Notes folder(s) hold the rolls + the catalog note (the AppleScript targets a
  named folder).
- Whether note bodies export cleanly as text/markdown (AppleScript `body` returns HTML —
  decide strip-in-script vs. convert-on-parse) and how reliably one note == one roll.
- Numbers CSV column → field mapping (defined once the export is in hand).
- Confirm seeded catalog contents so Phase 1 matching has the right baseline.
- NotePlan spot-check outcome (enriched rolls? → include; else discard).

## Explicitly out of scope (YAGNI)

- Kammerz MCP server.
- A batch-import review **screen** in the app (Approach C) — frontend work for a
  one-time job.
- Any database / SQL-level import.
