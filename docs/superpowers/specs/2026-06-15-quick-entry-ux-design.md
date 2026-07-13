# Quick Entry UX Refinement — Design (kammerz-ife)

> **Status:** Implemented — dated design record, kept as history. Current architecture decisions live in the [ADR index](../../adr/README.md).

## Problem

The standalone `/quick-entry` page (`frontend/src/routes/(app)/quick-entry/+page.svelte`)
is unrefined. Two issues:

1. **Roll selection is a bare `<Select>` dropdown.** With few rolls in flight, a
   visual list of the active rolls reads better than a hidden dropdown.
2. **The logging UI is a less-refined duplicate.** The roll-detail page
   (`rolls/[id]/+page.svelte`) already has a polished frame-logging surface — the
   `QuickAddBar` (Frame / f/ / Shutter / Lens / Save & Next / More) plus the
   `FrameStrip` film strip. The standalone page instead uses a bespoke
   `grid grid-cols-2 md:grid-cols-4` form and a plain "Previous Shots" text list.

Goal: bring the standalone page up to par by **reusing** the roll-detail components
and the rolls-list row, and add a visual roll picker — rather than authoring new UI.

## Decisions (locked with the user)

- **Scope:** Full refinement — visual roll picker **and** replace the bespoke
  form/list with the shared `QuickAddBar` + `FrameStrip`.
- **Picker layout:** Reuse the rolls-list row (full-width, film-strip edge,
  `roll_id`, status `Badge`, ledger metadata, `FrameCounter`).
- **Which rolls:** Active only — `loaded` / `shooting` / `shot`. Others (at-lab,
  scanned, archived, …) are excluded.
- **On select:** Collapse the picker to a single compact selected row + a `Change`
  button (re-expands the picker). The film strip + entry bar get full width.
- **Filled-frame click = no-op** (scope boundary): open slots seed the next frame;
  editing a past frame stays on the roll-detail page. No `ShotDialog` extraction.

## Architecture / Changes

### 1. New `RollRow.svelte` — shared presentational row

`frontend/src/lib/components/rolls/RollRow.svelte`. Extracted from the rolls-list
row markup (`rolls/+page.svelte`, currently lines ~185–218). Renders the shared
outer container (border, `bg-surface-raised`, `hover:border-accent/40`, padding),
the vertical `FilmStrip` edge, `roll_id`, status `Badge`, the wrapping ledger
metadata (camera / film / lens / date with dot separators), and the right-anchored
`FrameCounter`.

- Renders its interactive element via
  `<svelte:element this={href ? 'a' : 'button'}>` so one component serves both the
  rolls-list (link) and the picker (select button). Spreads `href` **or** `onclick`
  accordingly; `type="button"` when a button.
- Props: `roll: RollWithDetails`, optional `href?: string`,
  `onclick?: () => void`, optional `selected?: boolean` (accent border for the
  collapsed/selected state), and an optional `trailing` snippet for the row's
  right-most element (the `→` arrow on rolls-list, the `Change` button on the
  collapsed picker).
- **Rolls-list page is refactored to consume `RollRow`** with zero visual change
  (it provides `href` + the `→` arrow snippet). This is the proof the extraction is
  faithful.

### 2. New `buildFrameCells()` util

`frontend/src/lib/utils/frames.ts`. Pure function extracted from the roll-detail
`frameCells` `$derived.by` (`rolls/[id]/+page.svelte` ~line 278):

```ts
interface FrameCell { frameNumber: string; shot: Shot | null; isNext: boolean }
function buildFrameCells(shots: Shot[], frameCount: number | null): FrameCell[]
```

Maps shots onto numbered slots `1..n` (default 36 when `frameCount` is null),
flags the first empty slot as `isNext`, and appends extras (frame numbers outside
`1..n`, e.g. `"37"`, `"36A"`). Roll-detail is refactored to call it (no behavior
change). A `nextFrameNumber` is derived by the consumer as
`cells.find(c => c.isNext)?.frameNumber ?? ''`.

- **Unit test:** `frontend/src/lib/utils/frames.test.ts` (vitest) — covers empty
  roll, partial fill, full roll, null frame count → default 36, and extras. Matches
  the project convention that pure-logic utils gate inside `ci-frontend`.

### 3. Rewrite `/quick-entry/+page.svelte`

Replace the dropdown + bespoke form + "Previous Shots" list:

- **Roll picker** (when no roll selected, or `Change` pressed): active rolls
  (`['loaded','shooting','shot']`) rendered as `RollRow` select buttons. Section
  header in the ledger-line style. `EmptyState` (with a CTA to `/rolls/new`) when no
  active rolls exist.
- **Collapsed selected state** (roll selected): one `RollRow` with `selected` +
  a `Change` button (trailing snippet) that clears the selection back to the picker.
- **`QuickAddBar`** replaces the grid form. Wired exactly as roll-detail:
  `frameNumber={nextFrameNumber}`, `lensOptions`, `lensId={quickDefaultLensId}`,
  `isFixedLens`, `fixedLensLabel`, `saving`, `error`, `onsave={handleQuickAdd}`.
- **`FrameStrip`** replaces the "Previous Shots" list:
  `frames={buildFrameCells(shots, selectedRoll.frame_count)}`,
  `onselect={handleFrameSelect}`, `onaddextra`.
  - `QuickAddBar`'s Frame field is **display-only** — it renders the `frameNumber`
    prop (`{frameNumber || '—'}`) and never lets the user type one. The frame to log
    is therefore whatever the parent passes. The page passes
    `frameNumber={jumpFrame || nextFrameNumber}`.
  - `handleFrameSelect(frameNumber, shot)`: **open slot** (`shot === null`) → set
    `jumpFrame = frameNumber` (skip ahead / fill a gap — the bar's next save targets
    it). **Filled frame** (`shot !== null`) → **no-op**. `jumpFrame` resets to `''`
    after a successful save (in `handleQuickAdd`), so logging returns to sequential.
- **Preserved behavior:**
  - Query-param preselect (`?roll=42`).
  - Smart lens default cascade (`quickDefaultLensId`: fixed > roll.lens_id >
    camera.default_lens_id).
  - Roll-full "Mark as Shot" nudge (`showRollFullNudge` + `markRollShot` +
    `finishDate`).
  - Session counter (`sessionCount`, incremented in `handleQuickAdd`).
  - `⌘/Ctrl+Enter` save — `QuickAddBar` already owns this handler internally, so the
    page's current `handleKeydown` + `<svelte:window>` is dropped (no longer needed).

## Data flow

`listRolls()` → active filter → `RollRow` picker → select sets `selectedRollId`
→ `loadRollData` fetches `shots` → `buildFrameCells(shots, frame_count)` →
`FrameStrip`; `QuickAddBar.onsave` → `logShot` (existing util) → reload shots/rolls.
No new endpoints; `logShot` already supports the full `QuickShotInput`.

## Out of scope

- No edit-in-place for filled frames on Quick Entry (roll-detail covers it).
- No `ShotDialog` extraction.
- No backend / `entity` / `migration` / API changes.
- No redesign of `QuickAddBar` or `FrameStrip` internals.

## Testing & gates

- New `frames.test.ts` vitest unit test (gates in `ci-frontend`).
- `svelte-check` + `bun run build` (markup/types).
- e2e Playwright `smoke.spec.ts` (release binary).
- Manual: `just dev` → `/quick-entry` — pick a roll, log frames, verify collapse/
  Change, roll-full nudge, fixed-lens lock, query-param preselect.
- Re-verify the **rolls-list page is visually unchanged** after the `RollRow`
  extraction.
