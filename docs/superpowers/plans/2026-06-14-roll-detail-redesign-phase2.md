# Roll Detail Redesign — Phase 2 (Two-Pane Page) Implementation Plan

> **Status:** Implemented — dated design record, kept as history. Current architecture decisions live in the [ADR index](../../adr/README.md).

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. **Visual treatment is governed by the `design-system` skill — invoke it before writing component markup; this plan specifies structure, props, logic, and data flow, not pixel-final CSS.**

**Goal:** Replace the roll-detail page's stacked Status + Timeline + Development + Shots sections with a two-pane layout — film-strip metadata card → restored chevron status control → **Frames pane** (always-on quick-add bar + horizontal scrolling frame strip) + **Activity pane** (a journal rendering the Phase 1 `roll_events` log) — with development details edited from their journal events.

**Architecture:** Four new focused components (`RollStatusControl`, `FrameStrip`, `QuickAddBar`, `RollActivity`) under `frontend/src/lib/components/rolls/`, composed by a rewritten `+page.svelte`. The page keeps `DevelopmentSection` mounted (now driven only by its `autoPrompt` bridge, opened from chevron clicks and journal-event clicks) but drops its standalone visual section. All existing handlers (`handleStatusClick`, `updateStatus`, the dev-dialog bridge, shot CRUD, roll edit) are reused; the chevron logic moves into `RollStatusControl` unchanged. The activity feed is read-only and refreshes with every mutation via the existing `loadRollData`.

**Tech Stack:** SvelteKit, Svelte 5 runes, Tailwind 4 (Darkroom Ledger theme), Playwright (e2e). No frontend unit runner — verify with `svelte-check`, Playwright, and browser.

**Spec:** `docs/superpowers/specs/2026-06-13-roll-detail-redesign-design.md`. **Backend (Phase 1, merged PR #92):** `GET /api/rolls/{id}/detail` already returns `events`.

---

## File structure

- `frontend/src/lib/types/index.ts` — **modify**: add `RollEventType`, `RefKind`, `RollEvent`; add `events: RollEvent[]` to `RollDetail`.
- `frontend/src/lib/utils/shot-entry.ts` — **new**: `logShot()` shared save-and-advance helper used by the Quick Entry page and `QuickAddBar` (single source of truth for create-shot + suggest-next-frame).
- `frontend/src/lib/components/rolls/RollStatusControl.svelte` — **new**: the chevron transition control (extracted from `+page.svelte`), transition-only.
- `frontend/src/lib/components/rolls/FrameStrip.svelte` — **new**: horizontal scrolling sprocketed frame strip.
- `frontend/src/lib/components/rolls/QuickAddBar.svelte` — **new**: always-on inline quick-add with Save & Next + "more" expander.
- `frontend/src/lib/components/rolls/RollActivity.svelte` — **new**: the activity journal.
- `frontend/src/lib/utils/activity.ts` — **new**: `groupEventsByDay()` + label helpers for the journal (pure, unit-testable shape even without a runner).
- `frontend/src/routes/(app)/rolls/[id]/+page.svelte` — **modify**: two-pane layout, mount the four components, wire dev-edit-from-event, remove the Status + Timeline sections and the old Shots section markup.
- `frontend/src/lib/components/rolls/RollTimeline.svelte` — **delete** (replaced by the activity journal).
- `frontend/src/lib/utils/timeline.ts` — **trim**: keep `DateTarget`, `STATUS_DATE_TARGET`, `readDateTarget` (still used by `updateStatus`/`targetDate`); delete `buildRollTimeline`, `TimelineMilestone`, `MILESTONE_*`, `milestoneReached` if no longer referenced after the page rewrite.
- `frontend/src/routes/(app)/quick-entry/+page.svelte` — **modify**: call the shared `logShot()` helper (no behavior change).
- `frontend/tests/smoke.spec.ts` — **modify**: replace the kammerz-fxl/timeline e2e with redesign coverage.

---

## Task 1: Event TS types + `events` on `RollDetail`

**Files:**

- Modify: `frontend/src/lib/types/index.ts`

- [ ] **Step 1: Add the event types**

In `frontend/src/lib/types/index.ts`, add near the other roll types (the strings must match the Rust `#[serde(rename)]` values from `entity/src/roll_event.rs`):

```ts
export type RollEventType =
	| 'roll_loaded'
	| 'status_changed'
	| 'shot_logged'
	| 'shot_edited'
	| 'shot_deleted'
	| 'lab_dev_added'
	| 'lab_dev_edited'
	| 'lab_dev_removed'
	| 'self_dev_added'
	| 'self_dev_edited'
	| 'self_dev_removed';

export type RollEventRefKind = 'lab_dev' | 'self_dev' | 'shot';

export interface RollEvent {
	id: number;
	roll_id: number;
	event_type: RollEventType;
	from_status: RollStatus | null;
	to_status: RollStatus | null;
	ref_kind: RollEventRefKind | null;
	ref_id: number | null;
	summary: string;
	occurred_at: string;
	created_at: string;
}
```

- [ ] **Step 2: Add `events` to `RollDetail`**

```ts
export interface RollDetail {
	roll: RollWithDetails;
	shots: Shot[];
	shot_lens_pairs: [number, number][];
	lab_dev: DevelopmentLab | null;
	self_dev: DevelopmentSelf | null;
	dev_stages: DevStage[];
	events: RollEvent[];
}
```

- [ ] **Step 3: Verify + commit**

Run: `cd frontend && bun run check` → 0 errors.

```bash
git add frontend/src/lib/types/index.ts
git commit -m "feat(types): RollEvent + events on RollDetail (kammerz-3hq)"
```

---

## Task 2: Activity grouping util

**Files:**

- Create: `frontend/src/lib/utils/activity.ts`

The journal groups events by calendar day (descending) and rolls up same-day shot events into one summarized line (spec decision A). Events arrive newest-first from the backend.

- [ ] **Step 1: Write the util**

Create `frontend/src/lib/utils/activity.ts`:

```ts
import type { RollEvent } from '$lib/types';

/** A rendered journal row: either a single event, or a per-day rollup of shot events. */
export type ActivityRow =
	| { kind: 'event'; event: RollEvent }
	| { kind: 'shots'; day: string; count: number; latest: RollEvent };

export interface ActivityDay {
	day: string; // YYYY-MM-DD (from occurred_at)
	rows: ActivityRow[];
}

const SHOT_TYPES = new Set(['shot_logged', 'shot_edited', 'shot_deleted']);

/** The calendar day of an event (occurred_at is "YYYY-MM-DD HH:MM:SS"). */
function dayOf(e: RollEvent): string {
	return e.occurred_at.slice(0, 10);
}

/**
 * Group newest-first events into day buckets. Within a day, consecutive shot
 * events collapse into one `shots` rollup row ("N frame changes"); status/dev/
 * roll events stay as individual rows. Order within a day preserves the
 * incoming newest-first order.
 */
export function groupActivity(events: RollEvent[]): ActivityDay[] {
	const days: ActivityDay[] = [];
	let currentDay: ActivityDay | null = null;

	for (const e of events) {
		const day = dayOf(e);
		if (!currentDay || currentDay.day !== day) {
			currentDay = { day, rows: [] };
			days.push(currentDay);
		}
		if (SHOT_TYPES.has(e.event_type)) {
			const last = currentDay.rows[currentDay.rows.length - 1];
			if (last && last.kind === 'shots') {
				last.count += 1;
			} else {
				currentDay.rows.push({ kind: 'shots', day, count: 1, latest: e });
			}
		} else {
			currentDay.rows.push({ kind: 'event', event: e });
		}
	}
	return days;
}
```

- [ ] **Step 2: Verify + commit**

Run: `cd frontend && bun run check` → 0 errors.

```bash
git add frontend/src/lib/utils/activity.ts
git commit -m "feat(activity): groupActivity util for the journal (kammerz-3hq)"
```

---

## Task 3: Shared shot-entry helper

**Files:**

- Create: `frontend/src/lib/utils/shot-entry.ts`
- Modify: `frontend/src/routes/(app)/quick-entry/+page.svelte` (call the helper)

Factor the create-shot + suggest-next-frame logic so the Quick Entry page and the new `QuickAddBar` cannot diverge (spec).

- [ ] **Step 1: Write the helper**

Create `frontend/src/lib/utils/shot-entry.ts`:

```ts
import { createShot, suggestNextFrame } from '$lib/api/shots';

export interface QuickShotInput {
	rollId: number;
	frameNumber: string;
	aperture?: string;
	shutterSpeed?: string;
	lensId?: string; // '' = none
	date?: string; // '' = none
	location?: string;
	notes?: string;
}

/**
 * Create one shot and return the suggested next frame number. Shared by the
 * Quick Entry page and the inline QuickAddBar so save-and-advance behaves
 * identically. Throws on API error (caller shows the message). `date`/`location`
 * default to none — the Quick Entry page passes blanks; the QuickAddBar passes
 * them only when the "more" fields are filled.
 */
export async function logShot(input: QuickShotInput): Promise<string> {
	const lensIds = input.lensId ? [Number(input.lensId)] : [];
	await createShot({
		roll_id: input.rollId,
		frame_number: input.frameNumber.trim(),
		aperture: input.aperture?.trim() || null,
		shutter_speed: input.shutterSpeed?.trim() || null,
		date: input.date?.trim() || null,
		date_fuzzy: null,
		location: input.location?.trim() || null,
		gps_lat: null,
		gps_lon: null,
		notes: input.notes?.trim() || null,
		lens_ids: lensIds
	});
	try {
		return await suggestNextFrame(input.rollId);
	} catch {
		return '';
	}
}
```

- [ ] **Step 2: Use it in Quick Entry's `handleSave`**

In `frontend/src/routes/(app)/quick-entry/+page.svelte`, replace the inline `createShot({...})` + `suggestNextFrame` block inside `handleSave` with:

```ts
frameNumber = await import('$lib/utils/shot-entry').then(() => undefined) as never; // placeholder line — see below
```

…actually import at top: add `import { logShot } from '$lib/utils/shot-entry';` to the imports, then the body of `handleSave` becomes (preserving the existing session-count, success message, reload, focus, and lens-retention behavior):

```ts
async function handleSave() {
	if (!selectedRollId || !frameNumber.trim()) {
		error = 'Please select a roll and enter a frame number.';
		return;
	}
	error = '';
	saving = true;
	try {
		const nextFrame = await logShot({
			rollId: Number(selectedRollId),
			frameNumber,
			aperture,
			shutterSpeed,
			lensId: selectedLensId,
			notes
		});
		sessionCount++;
		lastSavedFrame = frameNumber.trim();
		successMessage = `Frame ${frameNumber} saved`;
		setTimeout(() => (successMessage = ''), 2000);
		[shots, rolls] = await Promise.all([listShotsForRoll(Number(selectedRollId)), listRolls()]);
		aperture = '';
		shutterSpeed = '';
		notes = '';
		frameNumber = nextFrame;
		setTimeout(() => {
			document.querySelector<HTMLInputElement>('[data-field="aperture"]')?.focus();
		}, 50);
	} catch (err) {
		error = err instanceof Error ? err.message : String(err);
	} finally {
		saving = false;
	}
}
```

Remove the now-unused direct `createShot`/`suggestNextFrame` imports **only if** no longer referenced elsewhere in the file (Grep first; `suggestNextFrame` is also used on roll change — keep that import if so).

- [ ] **Step 3: Verify + commit**

Run: `cd frontend && bun run check` → 0 errors. Then `just dev`, open `/quick-entry?roll=<id>`, log two frames, confirm Save & Next still clears aperture/shutter/notes, keeps lens, advances the frame, and shows the success flash.

```bash
git add frontend/src/lib/utils/shot-entry.ts frontend/src/routes/(app)/quick-entry/+page.svelte
git commit -m "refactor(shots): shared logShot helper for quick-entry + bar (kammerz-3hq)"
```

---

## Task 4: `RollStatusControl.svelte` (chevron transition control)

**Files:**

- Create: `frontend/src/lib/components/rolls/RollStatusControl.svelte`

Extract the existing chevron bar verbatim into a component. It is **transition-only** (no dates). It receives the flow + current status + a `hintFor` callback and emits `onmove`/`onchoosepath`; all decision logic stays in the parent's `handleStatusClick`.

- [ ] **Step 1: Write the component**

Create `frontend/src/lib/components/rolls/RollStatusControl.svelte`. Props:

```ts
interface Props {
	statusFlow: RollStatus[];
	currentStatus: RollStatus;
	currentStatusIdx: number;
	devPath: DevPath;
	pathLabel: string | null;
	hintFor: (status: RollStatus) => string;
	onmove: (status: RollStatus) => void;
	onchoosepath: (path: 'lab' | 'self') => void;
}
```

Move the existing markup from `+page.svelte` lines 966–1084 (the header with the `CircleHelp` help disclosure, the `pathLabel` line, the chevron `{#each statusFlow ...}` loop with its clip-path + state classes, and the undecided "Develop ⌄" dropdown calling `onchoosepath`). Internal `$state` for `showStatusHelp` and `showDevPathMenu` lives in the component. Replace `handleStatusClick(status)` calls with `onmove(status)`, `chooseDevPath(p)` with `onchoosepath(p)`, and `statusConfig[status].label` stays. **Do NOT** move `statusNotice`/`autoStatusNotice` — those stay in the parent (they belong to the page, rendered beneath the control). Import `statusConfig` from `$lib/utils/status`, `CircleHelp` from `lucide-svelte`, types from `$lib/types` and `$lib/utils/status` (`DevPath`).

> Use the design-system skill for the visual treatment, but **preserve** the existing chevron clip-path geometry and the three state classes (current `bg-accent text-surface`, past `bg-surface-overlay text-accent`, future `bg-surface-raised text-text-muted`) — the user explicitly wanted the chevrons back exactly as they were.

- [ ] **Step 2: Verify + commit**

Run: `cd frontend && bun run check` → 0 errors. (Wired into the page in Task 7; not rendered standalone yet.)

```bash
git add frontend/src/lib/components/rolls/RollStatusControl.svelte
git commit -m "feat(ui): RollStatusControl chevron transition control (kammerz-3hq)"
```

---

## Task 5: `FrameStrip.svelte`

**Files:**

- Create: `frontend/src/lib/components/rolls/FrameStrip.svelte`

Horizontal scrolling sprocketed strip of frame cells, sized to the roll's frame count, with a trailing **＋** for over-roll frames. Pure presentation; emits selection.

- [ ] **Step 1: Define the cell model + props**

```ts
interface FrameCell {
	frameNumber: string; // "1".."N", plus any extra/non-integer shots appended
	shot: Shot | null; // null = open slot
	isNext: boolean; // the next open frame to log (auto-scroll target + ring)
}
interface Props {
	frames: FrameCell[];
	onselect: (frameNumber: string, shot: Shot | null) => void;
	onaddextra: () => void;
}
```

The **parent** builds `frames` (Task 7) so the slot logic is testable and centralized:

- N = `roll.frame_count ?? <film-stock default or 36>`; slots `"1".."N"`.
- Map each logged shot to its slot by `frame_number`; shots whose `frame_number` isn't an integer in `1..N` (e.g. `"00"`, `"37"`, `"36A"`) are appended as extra cells after slot N.
- `isNext` = the first open (shot===null) slot in `1..N`, else none.

- [ ] **Step 2: Write the component**

Markup: a `<div>` with `overflow-x-auto`, sprocket rails top & bottom via the existing `.film-perfs-x` CSS class (see `app.css`), and a row of cells. Each cell: a `<button>` showing `frameNumber` (mono) and, when `shot` is set, a hint line (aperture/shutter or lens display); open cells are faint; `isNext` gets an accent ring. Click → `onselect(frameNumber, shot)`. A trailing `<button>` "＋" → `onaddextra()`. On mount and when `frames` changes, scroll the `isNext` cell into view (`scrollIntoView({ inline: 'center', block: 'nearest' })` inside an `$effect`, guarded so it only runs when the next-frame identity changes).

> design-system skill governs exact cell sizing/spacing/colors; preserve the film aesthetic (sprocket rails, mono frame numbers, amber accent for filled/next).

- [ ] **Step 3: Verify + commit**

Run: `cd frontend && bun run check` → 0 errors.

```bash
git add frontend/src/lib/components/rolls/FrameStrip.svelte
git commit -m "feat(ui): FrameStrip horizontal frame strip (kammerz-3hq)"
```

---

## Task 6: `QuickAddBar.svelte`

**Files:**

- Create: `frontend/src/lib/components/rolls/QuickAddBar.svelte`

Always-on inline entry pinned above the strip, pre-aimed at the next open frame, Save & Next (⌘/Ctrl+Enter), with a "⋯ more" expander for date/location/notes.

- [ ] **Step 1: Props**

```ts
interface Props {
	frameNumber: string; // current target (parent supplies next-open frame)
	lensOptions: { value: string; label: string; disabled?: boolean }[];
	lensId: string;
	isFixedLens: boolean;
	fixedLensLabel: string;
	saving: boolean;
	error: string;
	onsave: (entry: {
		frameNumber: string;
		aperture: string;
		shutterSpeed: string;
		lensId: string;
		date: string;
		location: string;
		notes: string;
	}) => void;
}
```

- [ ] **Step 2: Write the component**

Internal `$state` for `aperture`, `shutterSpeed`, `date`, `location`, `notes`, `showMore`, and a local editable `lensId` seeded from the prop. Layout: a compact row — frame label (mono, from `frameNumber`), `f/` input (`data-field="aperture"` for focus parity with Quick Entry), shutter input, lens `<Select>` (or fixed-lens read-only display), and a primary **Save & Next** button calling `onsave({...})`. A "⋯ more" toggle reveals `<DateInput>` (date), location input, and notes `<Textarea>`. `<svelte:window onkeydown>` handles ⌘/Ctrl+Enter → save when `frameNumber` is non-empty. After the parent confirms a save (parent re-seeds `frameNumber` and clears the bar by remounting or via a `seq` prop), clear `aperture`/`shutterSpeed`/`notes`, keep lens — mirror Quick Entry. Show `error` inline (`bg-red-500/15 text-red-400`).

> Reuse `Select`, `DateInput`, `Textarea`, `Input`, `Button` from `$lib/components/ui/`. design-system skill governs styling; keep it compact and prominent (it's the field-priority control).

- [ ] **Step 3: Verify + commit**

Run: `cd frontend && bun run check` → 0 errors.

```bash
git add frontend/src/lib/components/rolls/QuickAddBar.svelte
git commit -m "feat(ui): QuickAddBar always-on inline shot entry (kammerz-3hq)"
```

---

## Task 7: `RollActivity.svelte` (the journal)

**Files:**

- Create: `frontend/src/lib/components/rolls/RollActivity.svelte`

Render the grouped activity. Status events show from→to (with a distinct backward affordance); dev events are click-through to their editor; shot rollups show a quiet per-day line.

- [ ] **Step 1: Props**

```ts
interface Props {
	events: RollEvent[];
	onopendev: (refKind: 'lab_dev' | 'self_dev') => void;
}
```

- [ ] **Step 2: Write the component**

Use `groupActivity(events)` from `$lib/utils/activity`. Render `{#each days}` with a day header (`ledger-line` style) then `{#each day.rows}`:

- `kind: 'event'` + `event_type === 'status_changed'`: show the status move. Use `getStatusLabel(to_status)`; if `from_status`/`to_status` indices imply a backward move (compare via `allStatusOrder` from `$lib/utils/status`), render a distinct ↩ icon/label ("Moved back to X"); else "Status → X". Color the dot via `getStatusColor(to_status)`.
- `kind: 'event'` + `event_type` in (`lab_dev_added`,`lab_dev_edited`): a clickable row (button) → `onopendev('lab_dev')`; `self_dev_*` (added/edited) → `onopendev('self_dev')`. `*_removed` rows are non-clickable (record gone). Use the event `summary` as the label.
- `kind: 'event'` + `roll_loaded`: "Roll loaded" with a neutral dot.
- `kind: 'shots'`: a faint row, e.g. `{count} frame change{count>1?'s':''}` (uses `row.latest.occurred_at` time if shown).

Import `getStatusLabel`, `getStatusColor`, `allStatusOrder` from `$lib/utils/status`; `groupActivity` + types from the util.

> design-system skill governs the visual treatment (ledger-line day headers, dot colors from status tokens, mono timestamps, the ↩ backward affordance). Keep status & dev events prominent, shot rollups quiet.

- [ ] **Step 3: Verify + commit**

Run: `cd frontend && bun run check` → 0 errors.

```bash
git add frontend/src/lib/components/rolls/RollActivity.svelte
git commit -m "feat(ui): RollActivity journal (kammerz-3hq)"
```

---

## Task 8: Rewire `+page.svelte` into the two-pane layout

**Files:**

- Modify: `frontend/src/routes/(app)/rolls/[id]/+page.svelte`

- [ ] **Step 1: Add the events derived + frame-cell builder + dev-open helper**

In the script: keep all existing state/handlers. Add:

```ts
import RollStatusControl from '$lib/components/rolls/RollStatusControl.svelte';
import FrameStrip from '$lib/components/rolls/FrameStrip.svelte';
import QuickAddBar from '$lib/components/rolls/QuickAddBar.svelte';
import RollActivity from '$lib/components/rolls/RollActivity.svelte';
import type { RollEvent } from '$lib/types';
```

Add an `events` state, populated in `loadRollData` from `detail.events` (alongside shots/labDev/etc.):

```ts
let events: RollEvent[] = $state([]);
// ...in loadRollData, after fetching `detail`:
events = detail.events ?? [];
```

Build the frame cells (the slot logic from Task 5):

```ts
const DEFAULT_FRAMES = 36;
const frameCells = $derived.by(() => {
	const n = roll?.frame_count ?? DEFAULT_FRAMES;
	const byFrame = new Map<string, Shot>();
	for (const s of shots) byFrame.set(s.frame_number.trim(), s);
	const cells: { frameNumber: string; shot: Shot | null; isNext: boolean }[] = [];
	let nextAssigned = false;
	for (let i = 1; i <= n; i++) {
		const fn = String(i);
		const shot = byFrame.get(fn) ?? null;
		const isNext = !shot && !nextAssigned;
		if (isNext) nextAssigned = true;
		cells.push({ frameNumber: fn, shot, isNext });
		byFrame.delete(fn);
	}
	// Extras: any shot whose frame_number wasn't a 1..n slot (e.g. "37", "00", "36A").
	for (const [fn, shot] of byFrame) cells.push({ frameNumber: fn, shot, isNext: false });
	return cells;
});
const nextFrameNumber = $derived(frameCells.find((c) => c.isNext)?.frameNumber ?? '');
```

Add the dev-open-from-journal helper (reuses the `autoPrompt` bridge — see DevelopmentSection): clicking a lab/self dev journal event opens its editor. Since `autoPrompt` opens the dialog seeded for the **existing** record when one exists, a bare `{ kind }` is enough:

```ts
function openDevFromEvent(refKind: 'lab_dev' | 'self_dev') {
	devAutoPrompt = { kind: refKind === 'lab_dev' ? 'lab' : 'self' };
}
```

Add a quick-add save handler bridging `QuickAddBar` → the shared helper:

```ts
let quickSaving = $state(false);
let quickError = $state('');
async function handleQuickAdd(entry: {
	frameNumber: string; aperture: string; shutterSpeed: string;
	lensId: string; date: string; location: string; notes: string;
}) {
	if (!roll || !entry.frameNumber.trim()) return;
	quickError = '';
	quickSaving = true;
	try {
		await logShot({ rollId: roll.id, ...entry });
		await loadRollData('shot-add');
	} catch (err) {
		quickError = err instanceof Error ? err.message : String(err);
	} finally {
		quickSaving = false;
	}
}
```

Add `import { logShot } from '$lib/utils/shot-entry';`. Keep `openEditShotDialog` for editing a filled frame (FrameStrip `onselect` with a non-null shot calls it; an open-slot select can pre-fill the Add Shot dialog at that frame or focus the bar — open-slot → `openAddShotDialog()` with the frame pre-set).

- [ ] **Step 2: Replace the Status + Timeline + Development + Shots markup with the two-pane layout**

Remove the Status section (lines ~966–1084), the Timeline section (~1086–1095), and the standalone Shots section markup (~1115–1233 header/rows; keep the Add/Edit Shot `Dialog` and the roll-full nudge logic). Keep `<DevelopmentSection ... bind:autoPrompt={devAutoPrompt} .../>` **mounted but visually minimal** (it now only provides the lab/self dialogs + delete confirms; its inline record display is superseded by the journal — render it without the section header, or keep a compact "Development" affordance per design-system). Compose:

```svelte
<FadeIn delay={50}>
	<RollStatusControl
		{statusFlow} currentStatus={roll.status} {currentStatusIdx} {devPath} {pathLabel}
		hintFor={statusHint} onmove={handleStatusClick} onchoosepath={chooseDevPath}
	/>
	{#if statusNotice}<p class="mt-2 text-xs text-text-faint">{statusNotice}</p>{/if}
	{#if autoStatusNotice}<div class="mt-2"><InlineNotice bind:message={autoStatusNotice} seq={autoStatusNoticeSeq} /></div>{/if}
</FadeIn>

<FadeIn delay={100}>
	<div class="grid gap-6 lg:grid-cols-2">
		<section>
			<!-- Frames pane: header (Frames N/total + progress), QuickAddBar, FrameStrip -->
			<QuickAddBar
				frameNumber={nextFrameNumber}
				lensOptions={shotLensOptions} lensId={/* roll/camera default */ ''}
				isFixedLens={isFixedLensCamera} fixedLensLabel={fixedLens ? lensDisplayName(fixedLens) : ''}
				saving={quickSaving} error={quickError} onsave={handleQuickAdd}
			/>
			<FrameStrip frames={frameCells} onselect={handleFrameSelect} onaddextra={openAddShotDialog} />
		</section>
		<section>
			<!-- Activity pane -->
			<RollActivity {events} onopendev={openDevFromEvent} />
		</section>
	</div>
</FadeIn>
```

Add `handleFrameSelect(frameNumber, shot)`: `shot ? openEditShotDialog(shot) : openAddShotDialog(frameNumber)`. Extend `openAddShotDialog` to accept an optional `frame?: string` that pre-sets `shotFrameNumber` (default keeps the `suggestNextFrame` behavior).

> The ledger headers, two-pane responsive stacking (single column under `lg`), film-strip card retention, and FadeIn stagger are governed by the design-system skill. The metadata film-strip card (view + edit) is unchanged.

- [ ] **Step 3: Verify**

Run: `cd frontend && bun run check` → 0 errors. Then `just dev` and exercise the page (see Verification section). Fix any wiring issues.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/routes/(app)/rolls/[id]/+page.svelte
git commit -m "feat(ui): two-pane roll detail (status control + frames + activity) (kammerz-3hq)"
```

---

## Task 9: Remove dead code

**Files:**

- Delete: `frontend/src/lib/components/rolls/RollTimeline.svelte`
- Modify: `frontend/src/lib/utils/timeline.ts`

- [ ] **Step 1: Delete RollTimeline + trim timeline.ts**

`git rm frontend/src/lib/components/rolls/RollTimeline.svelte`. In `timeline.ts`, Grep the codebase for each export; **keep** `DateTarget`, `STATUS_DATE_TARGET`, `readDateTarget` (used by `updateStatus`/`targetDate`/`writeDateTarget` in `+page.svelte`); **delete** `buildRollTimeline`, `TimelineMilestone`, `RungState`, `LifecycleRung`, `buildRollLifecycle`, `MILESTONE_*`, `milestoneReached`, `MilestoneKey` only if Grep shows zero remaining references. Remove now-unused imports in `+page.svelte` (`buildRollTimeline`, `TimelineMilestone`).

- [ ] **Step 2: Verify + commit**

Run: `cd frontend && bun run check` → 0 errors (this catches any still-referenced export).

```bash
git add -A frontend/src/lib/components/rolls/ frontend/src/lib/utils/timeline.ts frontend/src/routes/(app)/rolls/[id]/+page.svelte
git commit -m "chore(ui): remove RollTimeline + dead timeline utils (kammerz-3hq)"
```

---

## Task 10: e2e coverage + final gate

**Files:**

- Modify: `frontend/tests/smoke.spec.ts`

- [ ] **Step 1: Replace the obsolete timeline e2e**

The `kammerz-fxl` timeline test asserts a "Timeline" heading + date-editor gating that no longer exist. Replace it with a redesign smoke test: create a roll, open it, assert the chevron status control is present (a "Move to …"/current chevron), the frame strip renders the expected number of slots, the quick-add bar is visible, and the activity journal shows a "Roll loaded" entry. Then log a shot via the quick-add bar and assert a frame fills + an activity entry appears. Model it on the existing `roll detail page loads without an infinite fetch loop (kammerz-8k5)` test's roll-creation pattern.

```ts
test('roll detail shows status control, frame strip, quick-add, and activity (kammerz-3hq)', async ({ page }) => {
	const created = await page.request.post(`${BASE}/api/rolls`, {
		data: { roll_id: `E2E-P2-${Date.now()}`, status: 'loaded', frame_count: 36 }
	});
	expect(created.ok()).toBeTruthy();
	const id: number = await created.json();

	await page.goto(`${BASE}/rolls/${id}`);
	await expect(page.getByRole('heading', { name: 'Status' })).toBeVisible();
	// Chevron control: a clickable current/next status.
	await expect(page.getByRole('button', { name: /Move to|Current status/i }).first()).toBeVisible();
	// Activity journal founding event.
	await expect(page.getByText(/Roll loaded/i)).toBeVisible();
	// Quick-add bar present (frame label for the next open frame).
	await expect(page.getByText(/Frame\b/).first()).toBeVisible();

	await page.request.delete(`${BASE}/api/rolls/${id}`);
});
```

Adjust selectors to the final markup during implementation (the exact accessible names come from the components).

- [ ] **Step 2: Run the full gate**

Run: `cd frontend && bun run check` (0/0), then from repo root `just ci`. Restore `frontend/build/.gitkeep` if wiped.

- [ ] **Step 3: Commit**

```bash
git add frontend/tests/smoke.spec.ts
git commit -m "test(e2e): roll detail redesign smoke (kammerz-3hq)"
```

---

## Verification (end-to-end, manual)

`just dev`, open a roll at each stage:

1. **Lab-Done roll** (seed TEST-0e5 if present, else create + advance): chevron control shows the lab flow with the current rung highlighted; clicking a future rung prompts for a date (DateConfirm), a past rung asks to confirm (ConfirmDialog), and a lab/self rung with no record opens the dev dialog. The activity journal shows roll_loaded + status changes + dev events; clicking a "Lab development" journal event opens the lab editor.
2. **Frames:** the strip shows 36 slots, the next open frame ringed and auto-centered; clicking a filled frame opens the edit dialog; clicking an open frame logs at that frame; ＋ appends an extra. The quick-add bar logs the next frame with Save & Next (⌘/Ctrl+Enter), clears per-shot fields, keeps lens, and the new frame appears in the strip + a shot rollup appears in the journal.
3. **Undecided roll** at `shot`: the Develop ▾ chooser appears in the control; picking Lab/Self opens the dev dialog and re-renders the flow.
4. No console errors; two-pane collapses to single column below `lg`.

## design-system skill

Invoke `design-system` before writing the markup for Tasks 4–8. Honor: ledger-line section headers, `Badge`/status tokens (never inline status pills), `hover:border-accent/40`, `FadeIn` staggering, film-perf rails, mono for frame numbers/dates/timestamps. Preserve the chevron geometry + state classes (user explicitly wanted them back).

## Issue bookkeeping (post-merge, on `main`)

- Close **kammerz-3hq** (`chore(beads): close kammerz-3hq after PR #N merge`).
- The epic **kammerz-06i** can then close (both phases done) — verify no other children remain open.
- Standard post-merge: GH CI is the gate now (repo public); `bd dolt push`, commit the jsonl mirror on `main`, prune the branch, confirm `main` CI green.

## Self-review notes

- **Spec coverage:** two-pane layout ✓ (T8), chevron control restored ✓ (T4), frame strip + extras ✓ (T5/T8), quick-add bar + more expander + Save&Next ✓ (T6/T3), activity journal with summarized shots + backward moves + dev click-through ✓ (T7/T2), events on detail type ✓ (T1), remove old sections + RollTimeline ✓ (T8/T9), Quick Entry shares logic ✓ (T3). fxl gating concern is obsolete (no inline future-date editor exists in the redesign).
- **Type consistency:** `RollEvent`/`RollEventType`/`RollEventRefKind` names match the Rust serde strings; `groupActivity`/`ActivityRow`/`ActivityDay` used consistently in T2→T7; `logShot`/`QuickShotInput` consistent in T3→T8; `FrameCell` shape consistent T5→T8.
- **Deferred-by-design:** exact CSS/markup is intentionally not pinned (design-system skill owns it at execution); this is not a placeholder but a division of responsibility per the plan header.
