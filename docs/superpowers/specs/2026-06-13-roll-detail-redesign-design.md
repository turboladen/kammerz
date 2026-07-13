# Roll Detail Page — UX Redesign

> **Status:** Implemented — dated design record, kept as history. Current architecture decisions live in the [ADR index](../../adr/README.md).

**Date:** 2026-06-13
**Status:** Design approved in brainstorming; pending spec review
**Supersedes:** PR #91 (lifecycle stepper) and the original framing of kammerz-06i

## Context

The roll detail page (`frontend/src/routes/(app)/rolls/[id]/+page.svelte`) accreted into a tall, low-density stack: a film-strip metadata card, a status section, a development section, and a shots section — with the primary in-the-field action ("Add Shot") buried at the very bottom. Recent work merged the old chevron Status bar and the dated Timeline into one vertical stepper (PR #91, unmerged), but in practice that lost the chevrons (which the user valued) and conflated two different jobs — _changing_ status and _recording when things happened_ — into one ambiguous control. Clicking a rung to "edit At Lab details" only offered to transition, never to edit.

The user wants a ground-up UX redo of this page:

- Keep the **film-strip aesthetic** for roll metadata (it works).
- Bring back **chevrons as a dedicated transition control** — one clear job: move the roll's status.
- Turn the lifecycle history into a **Jira-style activity journal** — a chronological record of everything that happened to the roll, including **backward** status moves and edits.
- Make **shot logging the fastest thing on the page**, high in the layout, for use while shooting in the field.
- Use the horizontal real estate well (the current page wastes it).

## Goals

- A roll detail page organized around **two panes**: the **frames** (shots) and the **activity journal**.
- A true, append-only **activity log** backed by a new `roll_events` table — the data the journal needs to communicate richly.
- **Zero-navigation shot logging** via an always-on quick-add bar.
- Restore the chevron status control as a distinct, unambiguous control.
- Make development and lifecycle details **editable from their journal events** (resolving the "can't edit At Lab details" frustration).

## Non-Goals

- No change to the catalog data model beyond adding `roll_events`.
- No change to the Quick Entry page's role as the dedicated fast-logging surface (the new inline bar mirrors it on the roll itself).
- The rolls **list** page (kammerz-jo4) is out of scope.

## Approved design decisions (from brainstorming)

1. **Journal = true activity log.** New `roll_events` table + migration + event emission + read endpoint. (Not a derived-only feed.)
2. **Layout = two-pane** ("Frames + Activity"), with the film-strip metadata card on top and the chevron status control directly beneath it.
3. **Frame strip = horizontal, scrolling**, showing every slot the roll holds (sized to `frame_count`, fallback to the film stock's default), with a trailing **＋** to append over-roll frames (37, 38…). Scrolls both directions; auto-centers the next open frame.
4. **Logging = always-on quick-add bar** pinned above the strip, pre-aimed at the next open frame, with **Save & Next** (⌘/Ctrl+Enter) and a **"⋯ more"** expander for date/location/notes.
5. **Journal shot scope = summarized.** The strip owns individual shots; the feed shows a quiet rolled-up line ("Frames 1–2 logged") per day so status & dev events stay prominent. (Underlying events stay per-shot; the rollup is a display concern.)

## Information architecture

Top to bottom:

1. **Metadata film-strip card** — reuse the existing card (`FilmStrip`, `FrameCounter`, roll id, `Badge`, camera/film/ISO/frames, loaded/finished, Edit). Largely unchanged.
2. **Status control** — the chevron bar returns, as a _control only_: clicking a forward rung advances; clicking a backward rung asks to confirm; lab/self rungs with no dev record open the dev dialog (existing `handleStatusClick` semantics preserved). It no longer carries dates inline.
3. **Two panes:**
   - **Frames (left):** the quick-add bar + the horizontal frame strip. Click a filled frame to edit; click an open frame to target it in the quick-add bar; ＋ appends an extra frame.
   - **Activity (right):** the reverse-chronological journal, grouped by day. Status changes (forward + backward) and dev events are prominent; shot logging appears as a summarized per-day line. Dev events are click-through to the dev editor.

On narrow screens the two panes stack (frames first, then activity).

## Backend — the activity log

### New entity + migration: `roll_events`

| Column        | Type                                      | Notes                                                                     |
| ------------- | ----------------------------------------- | ------------------------------------------------------------------------- |
| `id`          | i32 PK                                    |                                                                           |
| `roll_id`     | i32 FK → rolls (ON DELETE CASCADE)        |                                                                           |
| `event_type`  | enum (DeriveActiveEnum)                   | see taxonomy                                                              |
| `from_status` | Option<RollStatus>                        | status changes only                                                       |
| `to_status`   | Option<RollStatus>                        | status changes only                                                       |
| `ref_kind`    | Option<enum: lab_dev \| self_dev \| shot> | what `ref_id` points to                                                   |
| `ref_id`      | Option<i32>                               | the record this event is about (for click-through editing)                |
| `summary`     | String                                    | human text, e.g. "Lab development added · Richard Photo Lab"              |
| `occurred_at` | String (ISO datetime)                     | when it happened (defaults to now; backfilled events use the source date) |
| `created_at`  | String (ISO datetime)                     | row insertion time                                                        |

**Event type taxonomy (initial):** `roll_loaded`, `status_changed` (covers forward AND backward — direction is derivable from `from_status`/`to_status` index in the flow), `shot_logged`, `shot_edited`, `shot_deleted`, `lab_dev_added`, `lab_dev_edited`, `lab_dev_removed`, `self_dev_added`, `self_dev_edited`, `self_dev_removed`, `roll_edited` (metadata edits worth recording — keep conservative to avoid noise). Use a total `DeriveActiveEnum` so adding a variant is a compile error until handled (mirrors `RollStatus`).

### Event emission

Emit from the **service layer**, in the same transaction as the mutation where practical:

- `RollService` status/update path → `status_changed` when `status` changes (capture from/to); optional `roll_edited` for notable metadata edits.
- `ShotService` create/update/delete → `shot_logged` / `shot_edited` / `shot_deleted` (ref_kind=shot, ref_id=shot.id).
- `DevelopmentService` lab/self create/update/delete → the matching `*_dev_*` events (ref_kind + ref_id for click-through).

Keep emission centralized in a small helper (`RollEventService::record(...)`) so each call site is one line and the summary formatting lives in one place.

### No backfill

The app has never been deployed, so there is no existing roll data to seed events from — the migration only **creates the `roll_events` table** (no data step). Any rolls in a local dev DB simply start their journal from the next mutation; an empty journal on a pre-existing local roll is acceptable. (Separately, the user plans to **squash all migrations before release** — out of scope for this work, but it means this migration need not be defensive about historical data.)

### Endpoint

`GET /api/rolls/{id}/events` → `Vec<RollEvent>` ordered by `occurred_at` desc. Either a standalone route or folded into the existing `GET /api/rolls/{id}/detail` composite (preferred: add `events: Vec<RollEvent>` to `RollDetail` so the page still loads in one round-trip).

## Frontend — components

- **`RollStatusControl.svelte`** (new) — the chevron control, extracted and simplified to its single job (transition). Reuses `statusFlow`/`handleStatusClick`/`statusHint` from the current page. Restores the chevron clip-path styling the user liked (past/current/future states from the old bar). No dates.
- **`FrameStrip.svelte`** (new) — horizontal scrolling sprocketed strip; props: frames (sized to count) + shots + current target; emits `select(frameNumber)` (open → target in quick-add; filled → edit) and `addExtra()`. Sprocket rails via the existing `.film-perfs-x` CSS.
- **`QuickAddBar.svelte`** (new) — always-on inline entry pinned above the strip; fields f/ + shutter + lens, **Save & Next** (⌘/Ctrl+Enter), a **⋯ more** expander revealing date/location/notes. Reuses the Quick Entry page's save-and-advance logic (`suggestNextFrame`, field-retention) — factor the shared logic into `$lib/utils` so the Quick Entry page and this bar don't diverge.
- **`RollActivity.svelte`** (new) — the journal: fetches/receives `events`, groups by day, renders status (with a distinct ↩ affordance for backward moves), dev (click-through to the dev editor via `ref_kind`/`ref_id`), and a summarized per-day shot line. Reuses status colors from `status.ts`.
- **Removed:** `LifecycleStepper.svelte` (from the unmerged #91) and the old inline Status + Timeline + Development + Shots section markup in `+page.svelte`. `RollTimeline.svelte` was already deleted in #91; `timeline.ts`'s `buildRollLifecycle` is no longer needed — delete it (keep only what the new components still use, if anything).
- **Reused unchanged:** `FilmStrip`, `FrameCounter`, `Badge`, `DevelopmentSection`'s dialogs (the lab/self editors — now opened from journal events, not a standalone section), `Dialog`, `Button`, `DateConfirm`, `FadeIn`.

## Data flow

`GET /api/rolls/{id}/detail` (now incl. `events`) → page splits into the metadata card, `RollStatusControl`, the Frames pane (`QuickAddBar` + `FrameStrip`), and `RollActivity`. Any mutation (status change, quick-add save, dev edit from a journal event, frame edit) calls the existing service endpoint, the backend appends a `roll_event`, and the page reloads detail (`loadRollData`) so the journal and strip refresh together. The existing backend auto-sync (status advancing/reverting on shot/dev changes) emits its own `status_changed` events, so auto-syncs show up in the journal too (this also subsumes the kammerz-9xg transient-notice need — the journal is the persistent record).

## Phasing (each phase = its own implementation plan/PR)

1. **Phase 1 — backend activity log.** `roll_events` entity + migration (table only, no backfill), `RollEventService`, event emission across roll/shot/development services, `events` added to the detail endpoint, backend tests. Ships independently; the current UI ignores the new field.
2. **Phase 2 — page redesign.** The four new components, the two-pane layout, removal of the old sections, wiring dev-edit-from-event, e2e coverage. Depends on Phase 1.

This will be tracked as a **new epic** with Phase 1 / Phase 2 child beads (created after spec approval, serialized through one bd writer).

## PR #91 and bead disposition

- **PR #91 (lifecycle stepper):** superseded by this redesign — **close unmerged**. The frame strip + activity model replaces the stepper entirely.
- **kammerz-06i:** repurpose as the **epic** for this redesign — retitle it to the redesign and hang the Phase 1 / Phase 2 beads under it as children.
- **kammerz-4ec** (label↔date gap): moot under the new layout — close, resolved by the redesign.
- **kammerz-fxl** (future-rung date gating): already merged to `main`; its concern (don't back-fill unreached dates) is naturally satisfied — status dates now come from transitions/events, not editable future rungs.

## Testing

- **Backend:** `cargo test` — `roll_events` rows are created on status change (fwd + back), shot CRUD, and dev CRUD; the migration creates the table cleanly on a fresh DB; the detail endpoint includes `events` ordered correctly.
- **Frontend:** `bun run check`; Playwright e2e — the redesigned page renders the metadata card, chevron control, quick-add bar, frame strip (all slots + ＋), and the activity journal; quick-add logs the next frame and advances; a backward status move appears in the journal; a dev journal event opens the dev editor.
- **Manual:** `just dev` + browser against the seeded Lab-Done roll and a fresh undecided roll; verify field-logging speed end to end.
- **Gate:** `just ci` per phase; post the result as the PR gate comment.

## Risks / open considerations

- **Free-form `frame_number`:** shots store `frame_number` as a string (e.g. "00", "36A"). The strip maps integer frames 1..N to slots and renders non-integer/over-count frames as appended "extra" cells. Edge formatting to confirm during implementation.
- **Event noise vs. completeness:** `roll_edited` is deliberately conservative; we can widen the taxonomy later. Per-shot events exist but render summarized.
