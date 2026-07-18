# Activity-Based Roll Lifecycle — Design Spec

**Date:** 2026-07-18
**Status:** Approved (brainstorm session, user-validated)
**Related beads:** kammerz-11o3 (Edit Shot nav bug — ships independently, first)

## Problem

The single `status` enum forces a linear story onto a process whose tail is not
linear. After development, three activities — scanning, post-processing
(Lightroom/Photos), archiving negatives — overlap and reorder freely:

- Post-processing can start mid-scan and take days; "Scanned → Post-processed"
  labels only describe completed snapshots, not the in-between.
- There is no representation of waiting ("negs dry, not yet scanning") or of
  in-progress work ("scanning, three days in").
- Archiving is gated behind post-processed in the flow, but negatives can be
  binned before editing — or never (lab discarded them), which today has no
  representation at all.
- The stored status can contradict the underlying data, which has produced a
  recurring class of orphaned-status bugs (kammerz-e2u, kammerz-3wg,
  kammerz-afc…).

## Decision

Replace the stored status enum with **five activities whose states are derived
from date presence**. The backend derives all display/grouping values
server-side. (User explicitly chose full activities over a hybrid
linear-front/checklist-tail model.)

## Data model

No new tables. Activities map onto existing columns plus a few new ones on
`rolls`:

| Activity        | Start                       | Completion                     | Extra fields                                                    |
| --------------- | --------------------------- | ------------------------------ | --------------------------------------------------------------- |
| Shooting        | `date_loaded` (exists)      | `date_finished` (exists)       | —                                                               |
| Development     | existing lab/self dev records and their date fields | same           | lab-vs-self is a property of the activity, not separate statuses |
| Scanning        | `scan_started` (new)        | `date_scanned` (exists)        | —                                                               |
| Post-processing | `post_processing_started` (new) | `date_post_processed` (exists) | —                                                           |
| Archiving       | — (a moment, not a duration) | `date_archived` (exists)      | `archive_location` (new, text), `archive_na` (new, bool), `archive_na_reason` (new, text) |

**Derived state rules (no stored state):**

- Duration activities: start set + no completion → *in progress*; completion set
  → *done*; neither → *not started*.
- Archiving: `date_archived` set → *done*; `archive_na` → *N/A*; else *not
  done*. N/A and done are mutually exclusive (setting one clears the other).
- **Implicit completion:** an activity also counts as *done* when a
  strictly-later activity has any date. Chain: shooting → development →
  each of {scanning, post-processing, archiving}. Post-processing having
  started does **not** imply scanning is done (they overlap); none of the tail
  three imply each other.
- A roll is **Done** when all five activities are resolved (archiving N/A
  counts as resolved).

`status` column is dropped. The `RollStatus` enum, `LAB_FLOW`/`SELF_FLOW`,
`status-flows.json`, `tests/status_flows.rs`, and the frontend
`status.ts` flow machinery are retired with it.

## Derivation & display rules

Backend computes per roll (returned on list + detail responses; the frontend
never re-derives):

- **Per-activity state** (as above) with dates.
- **Badge labels:** *all* in-progress activities, rendered compound
  ("Scanning · Post-processing"). When nothing is in progress, a waiting label
  derived from the earliest unresolved activity: "To develop", "To scan",
  "To edit", "To archive". Terminal: "Done".
- **Group/sort key:** index of the earliest incomplete activity — the scalar
  used by the rolls list group-by, dashboard section ordering, and stats. Badge
  displays compound; grouping stays deterministic.

Consumers to update: dashboard sections ("In the Field" = shooting unresolved;
"In the Darkroom" = post-shooting, not Done; "Needs Attention" unchanged),
rolls list grouping, search results, stats distribution, CSV/import status
mapping, `Badge` component.

Existing behaviors preserved in new terms: dev auto-prompt (starting
Development opens the lab/self dev form; the record's dates drive the activity
state exactly as `date_received`/`date_processed` do today), roll-full nudge
(completes Shooting), backward moves = clearing a date behind a ConfirmDialog.
The "undecided path" concept disappears — Development is simply *not started*
until a record exists.

## Migration & backfill

One SeaORM migration: add columns, backfill, drop `status`. Backfill only
where derivation would otherwise regress visible progress, borrowing only
recorded dates (never fabricated), guarded and idempotent, using the
`pub const` + apply-fn test seam (kammerz-9fx pattern):

| Old status ≥ | If null, backfill |
| ------------- | ----------------- |
| shot | `date_finished` := max(shot dates) ?? `date_loaded` |
| scanned | `date_scanned` := `date_post_processed` ?? dev completion date |
| post-processed | (date_post_processed is prompted on transition; no action if genuinely absent — roll shows "To edit", user corrects on the board) |
| archived | `date_archived` := `date_post_processed` ?? `date_scanned` ?? dev completion |

At-lab/lab-done/developing/developed need no backfill — dev records already
carry those dates, and implicit completion covers shooting.

## Roll page

**Activity board** replaces both the status chevron bar and the "Lifecycle
dates" section (they merge — resolving their redundancy with the Activity
journal, which stays as the separate event-history log). One row per activity:
state, editable start/completion dates, archiving's location / N-A + reason.
Marking Development started routes through the existing dev dialog.

**Auto phase layout** — one page, no mode tabs; the derived phase reorders
sections:

- **Shooting phase:** quick entry + a "last shot" reference card (frame, f/,
  shutter, lens of the most recent shot — the "what settings did I just use?"
  case) at the top; activity board collapsed below.
- **Wrap-up phase** (development done onward): shots front and center
  (table view default), quick entry hidden.
- **Done:** compact summary first.

## Shots table + view-first dialog

- **Table view** of all shots (frame, f/, shutter, date, time, location, lens,
  notes), toggleable with the FrameStrip — zero-click reading while
  transcribing metadata into Lightroom/Photos.
- Clicking a shot (strip or table) opens the shot dialog **read-only with an
  Edit button** — in every phase. Edit mode retains < > navigation with
  auto-save-on-navigate (kammerz-11o3's decided behavior; that fix ships first
  against current code and this feature builds on it).

## Phasing

| # | Bead | Depends on |
| - | ---- | ---------- |
| ① | kammerz-11o3 — Edit Shot auto-save-on-navigate fix | — |
| ② | Backend: columns migration + backfill + derivation + API contract | — |
| ③ | Roll page: activity board + adaptive phase layout | ② |
| ④ | Shots table + view-first dialog | ②, ① |

## Out of scope

- Structured binder/sleeve entity for negative storage (archive_location is
  free text; a real storage hierarchy is a possible future feature).
- Inline cell editing in the shots table (view-first dialog covers
  corrections).
- Changes to Quick Entry page, print view beyond what the retired status
  requires mechanically.

On ship, promote the durable decision (derived activity states replace the
status enum) to an ADR and retire this spec per `workflow.md`.
