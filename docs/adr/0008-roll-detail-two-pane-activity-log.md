# ADR-0008: Roll detail page — chevron status control + append-only activity log (two-pane layout)

- **Status:** Accepted
- **Date:** 2026-06-13
- **Related:** `frontend/src/routes/(app)/rolls/[id]/+page.svelte`, `frontend/src/lib/components/rolls/{RollStatusControl,FrameStrip,QuickAddBar,RollActivity}.svelte`, `entity/src/roll_event.rs`, `src/services/roll_event_service.rs`, `migration/src/m20260614_000022_create_roll_events.rs`, `tests/roll_events.rs`, `.claude/rules/frontend-patterns.md` (Roll status progression section), beads kammerz-06i (epic), kammerz-45x/kammerz-3hq (phase children) — PR #92 (activity log, phase 1), PR #94 (two-pane page, phase 2)

## Context

The roll detail page's status chevron bar and its lifecycle timeline both
restated the same information — the roll's status history — as two separate
UI elements. An earlier attempt (PR #91, unmerged) tried to fuse them into one
vertical stepper, but that conflated two different jobs (_changing_ status vs
_recording what happened_) into one ambiguous control: clicking a rung to
"edit At Lab details" only offered a status transition, never an edit, and
backward moves / per-record edits (dev record edits, shot edits/deletes) had
nowhere to surface at all.

## Decision

Split the two jobs into two purpose-built surfaces, backed by a durable,
append-only activity log rather than a view derived solely from current
roll/shot/development state:

- **`roll_events`** (`entity/src/roll_event.rs`) is a real table, not a
  derived-only feed: `event_type` (`RollEventType`, a total `DeriveActiveEnum`
  mirroring `RollStatus` — `roll_loaded`, `status_changed`, `shot_logged/edited/deleted`,
  `lab_dev_added/edited/removed`, `self_dev_added/edited/removed`,
  `negatives_picked_up/waived`), `from_status`/`to_status` for status changes,
  `ref_kind`/`ref_id` (lab_dev/self_dev/shot) so the frontend can deep-link an
  event to its editor, a human `summary`, and `occurred_at`/`created_at`.
  `RollEventService::record(...)` (`src/services/roll_event_service.rs`) is the
  single emission point, called from the same transaction as the mutation in
  `routes/rolls.rs`, `routes/shots.rs`, and `routes/development.rs` — so status
  changes (forward _and_ backward), shot CRUD, and dev CRUD all leave a
  permanent record, including the backend's own auto-sync status changes
  (see the Data-driven status sync convention in `frontend-patterns.md`).
- **`RollStatusControl.svelte`** is now transition-only: it renders the chevron
  bar (forward = advance, backward = confirm, lab/self rungs with no dev
  record open the dev dialog) and carries no dates or history inline.
- **`RollActivity.svelte`** is the reverse-chronological journal: status
  changes (with a distinct affordance for backward moves), dev events
  (click-through to the lab/self dev editor via `ref_kind`/`ref_id`), and
  shot logging rolled up to a quiet per-day summary line (the frame strip
  owns individual shots; the journal stays readable rather than one line per
  frame).
- **Page layout** (`rolls/[id]/+page.svelte`) is now: film-strip metadata card
  → `RollStatusControl` → a Frames section combining an always-on
  `QuickAddBar` (zero-navigation shot logging, pre-aimed at the next open
  frame) + `FrameStrip` (horizontal scrolling frame strip, click a filled
  frame to edit / an open frame to target it / **+** to append over-roll
  frames) with `RollActivity` rendered alongside it → `DevelopmentSection`.
  `GET /api/rolls/{id}/detail` includes `events` so the page loads status,
  frames, and journal in one round-trip.

## Consequences

- Status history is no longer inferred/recomputed from current record state —
  it is a real, queryable log, which is what makes backward moves, per-record
  edits, and the auto-sync notices (`kammerz-9xg`) all show up truthfully and
  permanently instead of as a transient one-shot notice.
- Every future roll/shot/development mutation path that should be
  journal-visible must remember to call `RollEventService::record(...)` in its
  transaction — an emission site added without it fails silently (no event,
  no compile error), unlike the `RollStatus`/`RollEventType` enums themselves
  which are total and force a match update on a new variant.
- Two components now own what one page used to: `RollStatusControl` must
  never grow date/history rendering back into it, and `RollActivity` must
  never grow a status-transition control — the whole point of the split was
  giving each job exactly one place to live.
- Per-shot events are intentionally summarized (not itemized) in the journal
  by design — a roll with heavy shot-by-shot editing will show a rolled-up
  count, not a full per-shot diff; the underlying events remain per-shot in
  the table if finer detail is ever needed.
