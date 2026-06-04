# Lifecycle date capture & editing — design spec

**Bead:** kammerz-b08 · **Related:** kammerz-a7e (state-machine review) · **Date:** 2026-06-03

## Context / problem

On the roll-detail page, the **Timeline** shows "Received back" (the lab-done milestone)
with no date, and lifecycle dates are otherwise either silently stamped to *today* on a
status transition or edited in a separate form. But the day a milestone actually happened
is frequently not the day the user clicks the status chevron (e.g. you mark a roll "Lab
Done" days after it came back). Today's behavior is also **inconsistent** across transitions:

| → Status | Current behavior |
|---|---|
| Shot | "roll full" nudge captures an editable finish date ✅ |
| At Lab | auto-opens full Lab dialog (captures dropoff date) ✅ |
| **Lab Done** | **nothing — `date_received` stays null** ❌ |
| Developing | auto-opens full Self dialog (captures processed date) ✅ |
| Developed | nothing |
| Scanned / Post-processed / Archived | silently stamps **today**, no chance to adjust ⚠️ |

The "Received back" gap and the click-day≠event-day pain are the same root issue: clicking a
chevron should let you record the date the milestone *actually happened*, consistently.

## Goals

1. Every date-bearing **forward** transition lets the user record the real date (default
   today), including Lab Done → `date_received`.
2. Every lifecycle date is **editable** directly from the Timeline (set / change / clear).
3. One obvious home for dates (the Timeline), no redundant editing surfaces.
4. No regressions to status logic or the existing dev dialogs.

## Non-goals

- No schema or backend changes (all fields + update endpoints already exist).
- No redesign of the status state machine / dev-path flows — captured separately in
  **kammerz-a7e**. (This feature deliberately drops the "Skip" affordance rather than
  modeling an "advance-but-no-date" path.)
- At Lab / Developing dialogs are unchanged.

## Data model (unchanged — reused)

Every target date already exists with an existing update endpoint:

| Milestone | Record · field | Update path |
|---|---|---|
| Loaded | `roll.date_loaded` | `PUT /api/rolls/:id` (`updateRoll`) |
| Finished shooting | `roll.date_finished` | `updateRoll` |
| Dropped off at lab | `development_lab.date_dropped_off` | lab-dev update |
| **Received back** | `development_lab.date_received` | lab-dev update |
| Developed | `development_self.date_processed` | self-dev update |
| Scanned | `roll.date_scanned` | `updateRoll` |
| Post-processed | `roll.date_post_processed` | `updateRoll` |
| Archived | `roll.date_archived` | `updateRoll` |

## Design

### 1. Timeline milestones carry a write target

`frontend/src/lib/utils/timeline.ts` — extend `TimelineMilestone` with a `target` so both the
inline editor and the transition prompt know where each date lives:

```ts
export type DateTarget =
  | { kind: 'roll'; field: 'date_loaded' | 'date_finished' | 'date_scanned'
        | 'date_post_processed' | 'date_archived' }
  | { kind: 'lab';  field: 'date_dropped_off' | 'date_received' }
  | { kind: 'self'; field: 'date_processed' };

export interface TimelineMilestone {
  key: string;
  label: string;
  date: string | null;
  target: DateTarget;
  editable: boolean; // false when the source record doesn't exist yet
}
```

`buildRollTimeline` already knows each milestone's source, so this is additive. `editable` is
true for roll milestones always; for lab/self milestones it's true only when the corresponding
dev record exists (it always does on the lab/self path, which is the only time those rows show).

### 2. `DateConfirm.svelte` — shared date-pick UI

A single small component used by both the inline editor and the transition prompt. Props:
`title`, `value` (seed, default today), callbacks `onconfirm(date)`, `oncancel`, and (inline
mode only) `onclear`. Built from the existing `DateInput` inside a small popover/`ConfirmDialog`.
Buttons: **Confirm** (primary), **Cancel**; inline edit adds **Clear** (sets null). No "Skip".

### 3. `RollTimeline.svelte` — extracted, inline-editable

The Timeline currently renders inline in `[id]/+page.svelte` (~lines 792–814) which is already
~900 lines; extract it. Props: `milestones: TimelineMilestone[]`, `onedit(milestone, date|null)`.
Keeps the existing dot + dashed-line look; each row adds an edit affordance:

- has a date → show date + pencil; click → `DateConfirm` (edit) with Clear.
- no date (`editable`) → show "Set date" affordance; click → `DateConfirm` (set).
- not editable → render "—" as today (read-only).

On confirm/clear, calls `onedit`; the page routes by `target.kind` to `updateRoll` /
lab-dev-update / self-dev-update, then `loadRollData()`. **Editing a date never changes
status** — `auto_sync_status` keys off shots / dev-record existence, not these dates.

### 4. Confirm-on-transition

Generalize the existing auto-stamp in `updateStatus` (`[id]/+page.svelte` ~471). Replace the
roll-only `STATUS_DATE_FIELD` map with a `STATUS_DATE_TARGET: Partial<Record<RollStatus,
DateTarget>>` covering: `lab-done`→lab `date_received`, `developed`→self `date_processed`,
`scanned`/`post-processed`/`archived`→roll fields. (`shot` keeps its existing "roll full" nudge,
which already implements confirm-on-transition for `date_finished`; optionally refactor it to
reuse `DateConfirm`, but no behavior change.)

`handleStatusClick` flow:
- **Backward** move → existing `ConfirmDialog` (unchanged; dates untouched).
- **Forward** into a status with a `STATUS_DATE_TARGET` whose **target date is empty** → open
  `DateConfirm` (default today) instead of stamping. **Confirm(date)** → advance status
  (`updateRoll {status}`) *and* write `date` to the target record (roll/lab/self). **Cancel** →
  do nothing (status does not change; misclick is recoverable).
- Forward into a status with no target, or whose target date is already set → advance directly
  (current behavior), including the `at-lab`/`developing` dev-dialog auto-prompt.

Edge cases: `lab-done` requires a lab-dev record — on the lab path it always exists (created at
`at-lab`); if absent, fall through to a plain advance. `developed`'s `date_processed` is usually
already set at `developing`, so the prompt is skipped (forward+empty rule).

### 5. Roll Edit form drops its date pickers

Remove the five `DateInput`s (Date Loaded / Finished Shooting / Scanned / Post-processed /
Archived) and their `editDate*` state + save wiring (`[id]/+page.svelte` ~55–59, 544–548,
564–568, 647–653). Dates now live solely in the Timeline. Keep **Fuzzy Date** and all non-date
fields (roll ID, camera, film stock, lens, frame count, push/pull, notes).

## Files

**New**
- `frontend/src/lib/components/ui/DateConfirm.svelte`
- `frontend/src/lib/components/rolls/RollTimeline.svelte`

**Modified**
- `frontend/src/lib/utils/timeline.ts` — `DateTarget` + `target`/`editable` on milestones.
- `frontend/src/routes/(app)/rolls/[id]/+page.svelte` — `STATUS_DATE_TARGET`; `handleStatusClick`/
  `updateStatus` open `DateConfirm` on forward+empty; render `<RollTimeline onedit=…>`; route
  date saves to roll/lab/self updates; remove edit-form date fields + dead state.
- Reuse existing lab/self dev update API helpers (as used by `DevelopmentSection.svelte`).

**No backend changes.**

## Verification

- `cargo test -p kammerz` (unchanged backend → still green).
- `bun run check` (0/0) + `bun run build`.
- Browser (`just dev` → `:5273`), lab path: At Lab (dialog) → **Lab Done → DateConfirm,
  back-date it → Timeline shows `date_received`** → Scanned/Post/Archived each prompt → inline-edit
  a Timeline date and **Clear** one → confirm **status is unaffected** by date edits and the roll
  Edit form has **no** date pickers. Confirm Cancel on a forward prompt leaves status unchanged.
