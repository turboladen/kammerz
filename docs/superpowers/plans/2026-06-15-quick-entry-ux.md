# Quick Entry UX Refinement Implementation Plan

> **Status:** Implemented — dated design record, kept as history. Current architecture decisions live in the [ADR index](../../adr/README.md).

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the standalone `/quick-entry` page's bare dropdown + bespoke form with a visual roll picker and the same `QuickAddBar` + `FrameStrip` logging UI the roll-detail page already uses.

**Architecture:** Extract two shared pieces — a presentational `RollRow` component (from the rolls-list row) and a pure `buildFrameCells` util (from the roll-detail frame mapping) — then rewrite the Quick Entry page to consume them plus the existing `QuickAddBar`/`FrameStrip` components. Frontend-only; no backend, API, or migration changes.

**Tech Stack:** SvelteKit, Svelte 5 runes (`$state`/`$derived`/`$effect`/`$props`/`$bindable`), TypeScript, Tailwind CSS 4, vitest (unit), Playwright (e2e). Package manager: Bun.

---

## File Structure

- **Create** `frontend/src/lib/utils/frames.ts` — pure frame-cell mapping (`buildFrameCells`, `nextExtraFrameNumber`, `DEFAULT_FRAMES`).
- **Create** `frontend/src/lib/utils/frames.test.ts` — vitest unit tests for the above.
- **Create** `frontend/src/lib/components/rolls/RollRow.svelte` — shared presentational roll row (link / button / static).
- **Modify** `frontend/src/routes/(app)/rolls/[id]/+page.svelte` — use `buildFrameCells` instead of the inline `$derived.by` + local `DEFAULT_FRAMES`.
- **Modify** `frontend/src/routes/(app)/rolls/+page.svelte` — render rows via `RollRow` (zero visual change); drop now-unused `Badge`/`FilmStrip`/`FrameCounter` imports.
- **Modify** `frontend/src/routes/(app)/quick-entry/+page.svelte` — full rewrite (picker + collapse + `QuickAddBar` + `FrameStrip`).

---

## Task 1: Extract `buildFrameCells` util (TDD)

**Files:**

- Create: `frontend/src/lib/utils/frames.ts`
- Test: `frontend/src/lib/utils/frames.test.ts`
- Modify: `frontend/src/routes/(app)/rolls/[id]/+page.svelte`

- [ ] **Step 1: Write the failing test**

Create `frontend/src/lib/utils/frames.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { buildFrameCells, nextExtraFrameNumber, DEFAULT_FRAMES } from './frames';
import type { Shot } from '$lib/types';

function shot(frame_number: string, over: Partial<Shot> = {}): Shot {
	return {
		id: 1,
		roll_id: 1,
		frame_number,
		aperture: null,
		shutter_speed: null,
		date: null,
		date_fuzzy: null,
		location: null,
		gps_lat: null,
		gps_lon: null,
		notes: null,
		created_at: '',
		updated_at: '',
		...over
	};
}

describe('buildFrameCells', () => {
	it('defaults to 36 frames when frameCount is null', () => {
		const cells = buildFrameCells([], null);
		expect(cells).toHaveLength(DEFAULT_FRAMES);
		expect(cells[0]).toEqual({ frameNumber: '1', shot: null, isNext: true });
		expect(cells[1].isNext).toBe(false);
	});

	it('marks the first empty slot as next with shots filled in order', () => {
		const cells = buildFrameCells([shot('1'), shot('2')], 24);
		expect(cells).toHaveLength(24);
		expect(cells[0].shot?.frame_number).toBe('1');
		expect(cells[1].shot?.frame_number).toBe('2');
		expect(cells[2]).toMatchObject({ frameNumber: '3', isNext: true });
	});

	it('has no next slot when the roll is full', () => {
		const shots = Array.from({ length: 3 }, (_, i) => shot(String(i + 1)));
		const cells = buildFrameCells(shots, 3);
		expect(cells).toHaveLength(3);
		expect(cells.some((c) => c.isNext)).toBe(false);
	});

	it('appends extras (frame numbers outside 1..n) after the numbered slots', () => {
		const cells = buildFrameCells([shot('1'), shot('37')], 36);
		expect(cells).toHaveLength(37);
		expect(cells[36]).toMatchObject({ frameNumber: '37', isNext: false });
		expect(cells[36].shot?.frame_number).toBe('37');
	});

	it('treats whitespace-padded frame numbers as their trimmed slot', () => {
		const cells = buildFrameCells([shot(' 2 ')], 4);
		expect(cells[1].shot?.frame_number).toBe(' 2 ');
		expect(cells[0].isNext).toBe(true); // slot 1 is the first open one
	});
});

describe('nextExtraFrameNumber', () => {
	it('returns frameCount + 1 when no shots exceed the count', () => {
		expect(nextExtraFrameNumber([shot('1')], 36)).toBe('37');
	});

	it('returns one past the highest numeric frame when extras exist', () => {
		expect(nextExtraFrameNumber([shot('36'), shot('37')], 36)).toBe('38');
	});

	it('defaults the base to 36 when frameCount is null', () => {
		expect(nextExtraFrameNumber([], null)).toBe('37');
	});

	it('ignores non-numeric frame numbers', () => {
		expect(nextExtraFrameNumber([shot('36A')], 36)).toBe('37');
	});
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd frontend && bun run test:unit -- frames`
Expected: FAIL — `Failed to resolve import "./frames"` / module not found.

- [ ] **Step 3: Write the implementation**

Create `frontend/src/lib/utils/frames.ts`:

```ts
import type { Shot } from '$lib/types';

export interface FrameCell {
	frameNumber: string;
	shot: Shot | null;
	isNext: boolean;
}

/** Default frame count assumed when a roll's frame_count is null. */
export const DEFAULT_FRAMES = 36;

/**
 * Map a roll's shots onto numbered frame slots `1..n` (n = frameCount, or
 * DEFAULT_FRAMES when null). The first empty slot is flagged `isNext`. Shots whose
 * frame number falls outside `1..n` (e.g. "37", "36A" over-rolls) are appended after
 * the numbered slots, never flagged next. Shared by the roll-detail page and the
 * Quick Entry page so the film strip behaves identically in both.
 */
export function buildFrameCells(shots: Shot[], frameCount: number | null): FrameCell[] {
	const n = frameCount ?? DEFAULT_FRAMES;
	const byFrame = new Map<string, Shot>();
	for (const s of shots) byFrame.set(s.frame_number.trim(), s);

	const cells: FrameCell[] = [];
	let nextAssigned = false;
	for (let i = 1; i <= n; i++) {
		const fn = String(i);
		const shot = byFrame.get(fn) ?? null;
		const isNext = !shot && !nextAssigned;
		if (isNext) nextAssigned = true;
		cells.push({ frameNumber: fn, shot, isNext });
		byFrame.delete(fn);
	}
	// Extras: any shot whose frame_number wasn't a 1..n slot. Keyed by the trimmed
	// frame number, matching the roll-detail page's original behavior.
	for (const [fn, shot] of byFrame) cells.push({ frameNumber: fn, shot, isNext: false });
	return cells;
}

/**
 * Frame number for the next over-roll / extra frame: one past whichever is larger of
 * the roll's frame count and the highest numeric frame already logged. Used by the
 * Quick Entry "+" button to target a frame the sequential next-slot logic can't reach
 * (e.g. a 37th frame on a 36-exposure roll). Non-numeric frame numbers are ignored.
 */
export function nextExtraFrameNumber(shots: Shot[], frameCount: number | null): string {
	let max = frameCount ?? DEFAULT_FRAMES;
	for (const s of shots) {
		const num = parseInt(s.frame_number.trim(), 10);
		if (!Number.isNaN(num) && num > max) max = num;
	}
	return String(max + 1);
}
```

NOTE: `buildFrameCells` appends extras using the trimmed map key (`fn`) as the cell's `frameNumber`, exactly as the roll-detail page's original `$derived.by` did — so the refactor is behavior-preserving for roll-detail.

- [ ] **Step 4: Run test to verify it passes**

Run: `cd frontend && bun run test:unit -- frames`
Expected: PASS — all 9 tests green.

- [ ] **Step 5: Refactor roll-detail to use the util**

In `frontend/src/routes/(app)/rolls/[id]/+page.svelte`:

Add to the existing import block (near the other `$lib/utils` imports):

```ts
import { buildFrameCells } from '$lib/utils/frames';
```

Replace the local `DEFAULT_FRAMES` const and the `frameCells` `$derived.by` block (currently around lines 277–296, starting `const DEFAULT_FRAMES = 36;` through the closing `});` of `$derived.by`) with:

```ts
// Frame cells: map shots onto numbered slots, extras appended after (shared util).
const frameCells = $derived(buildFrameCells(shots, roll?.frame_count ?? null));
```

Leave the existing `nextFrameNumber` derived line immediately below it unchanged:

```ts
const nextFrameNumber = $derived(frameCells.find((c) => c.isNext)?.frameNumber ?? '');
```

- [ ] **Step 6: Verify roll-detail still type-checks and builds**

Run: `cd frontend && bun run check`
Expected: PASS — no svelte-check errors. (`DEFAULT_FRAMES` is no longer referenced in this file; confirm no "unused" complaint remains.)

- [ ] **Step 7: Commit**

```bash
git add frontend/src/lib/utils/frames.ts frontend/src/lib/utils/frames.test.ts "frontend/src/routes/(app)/rolls/[id]/+page.svelte"
git commit -m "refactor(frames): extract buildFrameCells util with tests (kammerz-ife)"
```

---

## Task 2: Extract `RollRow` shared component

**Files:**

- Create: `frontend/src/lib/components/rolls/RollRow.svelte`
- Modify: `frontend/src/routes/(app)/rolls/+page.svelte`

- [ ] **Step 1: Create the component**

Create `frontend/src/lib/components/rolls/RollRow.svelte`:

```svelte
<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { RollWithDetails } from '$lib/types';
	import Badge from '$lib/components/ui/Badge.svelte';
	import FilmStrip from '$lib/components/ui/FilmStrip.svelte';
	import FrameCounter from '$lib/components/ui/FrameCounter.svelte';

	interface Props {
		roll: RollWithDetails;
		/** Render as a link (rolls list). Mutually exclusive with onclick. */
		href?: string;
		/** Render as a select button (picker). Mutually exclusive with href. */
		onclick?: () => void;
		/** Accent border for the chosen/selected state (collapsed picker). */
		selected?: boolean;
		/** Right-most element: the → arrow (list) or a Change button (collapsed picker). */
		trailing?: Snippet;
	}

	let { roll, href, onclick, selected = false, trailing }: Props = $props();

	const interactive = $derived(!!href || !!onclick);
	// href → <a>, onclick → <button>, neither → static <div> (so a trailing <button>
	// inside the collapsed row isn't an invalid nested button).
	const tag = $derived(href ? 'a' : onclick ? 'button' : 'div');
</script>

<svelte:element
	this={tag}
	{href}
	{onclick}
	type={tag === 'button' ? 'button' : undefined}
	class="group relative flex w-full items-center gap-x-3 overflow-hidden rounded-lg border bg-surface-raised py-2.5 pl-5 pr-4 text-left transition-all duration-150 {selected
		? 'border-accent'
		: 'border-border'} {interactive ? 'hover:border-accent/40 hover:-translate-y-px' : ''}"
>
	<FilmStrip orientation="vertical" />
	<span class="shrink-0 font-mono text-sm font-semibold">{roll.roll_id}</span>
	<Badge status={roll.status} />
	<!-- Ledger metadata: flows left-to-right with dot separators, wraps gracefully. -->
	<div class="flex flex-wrap items-center gap-x-2 gap-y-0.5">
		{#if roll.camera_brand}
			<span class="text-sm text-text-muted">{roll.camera_brand} {roll.camera_model}</span>
		{:else}
			<span class="text-sm italic text-text-faint">No camera</span>
		{/if}
		{#if roll.film_stock_brand}
			<span class="text-xs text-text-faint" aria-hidden="true">&middot;</span>
			<span class="text-xs text-text-faint">{roll.film_stock_brand} {roll.film_stock_name}</span>
		{/if}
		{#if roll.lens_brand}
			<span class="text-xs text-text-faint" aria-hidden="true">&middot;</span>
			<span class="text-xs text-text-faint">{roll.lens_brand} {roll.lens_name}</span>
		{/if}
		{#if roll.date_loaded}
			<span class="text-xs text-text-faint" aria-hidden="true">&middot;</span>
			<span class="font-mono text-xs text-text-faint">{roll.date_loaded}</span>
		{/if}
	</div>
	<!-- Right-anchored: frame counter + optional trailing element. -->
	<div class="ml-auto flex shrink-0 items-center gap-2.5">
		<FrameCounter current={roll.shot_count} total={roll.frame_count} />
		{#if trailing}{@render trailing()}{/if}
	</div>
</svelte:element>
```

- [ ] **Step 2: Refactor the rolls-list page to use it**

In `frontend/src/routes/(app)/rolls/+page.svelte`:

Remove these three now-unused imports (they live in `RollRow` now):

```ts
import Badge from '$lib/components/ui/Badge.svelte';
import FilmStrip from '$lib/components/ui/FilmStrip.svelte';
import FrameCounter from '$lib/components/ui/FrameCounter.svelte';
```

Add this import alongside the other component imports:

```ts
import RollRow from '$lib/components/rolls/RollRow.svelte';
```

Replace the row markup — the entire `<a href="/rolls/{roll.id}" ...> ... </a>` block inside the `<FadeIn>` (currently lines ~185–218) — with:

```svelte
<RollRow {roll} href="/rolls/{roll.id}">
	{#snippet trailing()}
		<span class="text-xs text-text-faint opacity-0 transition-opacity group-hover:opacity-100">&rarr;</span>
	{/snippet}
</RollRow>
```

(The `<FadeIn delay={...}>` wrapper and the `{#each groupRolls as roll, i}` loop stay exactly as they are.)

- [ ] **Step 3: Verify it type-checks and builds**

Run: `cd frontend && bun run check`
Expected: PASS — no svelte-check errors, no unused-import warnings.

- [ ] **Step 4: Visually confirm the rolls list is unchanged**

Run: `just dev` (separate terminal), open `http://localhost:5273/rolls`.
Expected: rows look identical to before — film-strip edge, roll id, status badge, metadata, frame counter, and the `→` that fades in on hover. Stop `just dev` after confirming.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/lib/components/rolls/RollRow.svelte "frontend/src/routes/(app)/rolls/+page.svelte"
git commit -m "refactor(rolls): extract RollRow component (kammerz-ife)"
```

---

## Task 3: Rewrite the Quick Entry page

**Files:**

- Modify (full rewrite): `frontend/src/routes/(app)/quick-entry/+page.svelte`

- [ ] **Step 1: Replace the file contents**

Overwrite `frontend/src/routes/(app)/quick-entry/+page.svelte` with:

```svelte
<script lang="ts">
	import { page } from '$app/state';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import FadeIn from '$lib/components/ui/FadeIn.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import DateInput from '$lib/components/ui/DateInput.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import RollRow from '$lib/components/rolls/RollRow.svelte';
	import QuickAddBar from '$lib/components/rolls/QuickAddBar.svelte';
	import FrameStrip from '$lib/components/rolls/FrameStrip.svelte';
	import { Film } from 'lucide-svelte';
	import { listRolls, updateRoll } from '$lib/api/rolls';
	import { listCameras } from '$lib/api/cameras';
	import { listLenses } from '$lib/api/lenses';
	import { listLensMounts } from '$lib/api/lens-mounts';
	import { listShotsForRoll } from '$lib/api/shots';
	import { logShot } from '$lib/utils/shot-entry';
	import { buildLensOptions, lensDisplayName } from '$lib/utils/lens';
	import { buildFrameCells, nextExtraFrameNumber } from '$lib/utils/frames';
	import { todayLocal, dateFieldError } from '$lib/utils/date';
	import type { RollWithDetails, Camera, Lens, LensMount, Shot } from '$lib/types';

	let rolls: RollWithDetails[] = $state([]);
	let cameras: Camera[] = $state([]);
	let allLenses: Lens[] = $state([]);
	let lensMounts: LensMount[] = $state([]);
	let selectedRollId = $state('');
	let shots: Shot[] = $state([]);
	let loading = $state(true);
	let error = $state('');
	let sessionCount = $state(0);

	// QuickAddBar save state
	let quickSaving = $state(false);
	let quickError = $state('');

	// When set (via a FrameStrip click on an open slot or the "+" extra button), the
	// QuickAddBar logs THIS frame instead of the sequential next one. Cleared after a
	// successful save so logging returns to sequential.
	let jumpFrame = $state('');

	// Active rolls only — the ones you'd realistically log frames against.
	const activeStatuses = ['loaded', 'shooting', 'shot'];
	const activeRolls = $derived(rolls.filter((r) => activeStatuses.includes(r.status)));

	const selectedRoll = $derived(rolls.find((r) => String(r.id) === selectedRollId) ?? null);

	const selectedCamera = $derived(
		selectedRoll?.camera_id ? (cameras.find((c) => c.id === selectedRoll.camera_id) ?? null) : null
	);

	const isFixedLens = $derived(
		selectedCamera ? lensMounts.some((m) => m.id === selectedCamera.lens_mount_id && m.name === 'Fixed Lens') : false
	);
	const fixedLens = $derived(
		isFixedLens && selectedCamera?.default_lens_id
			? (allLenses.find((l) => l.id === selectedCamera.default_lens_id) ?? null)
			: null
	);

	const lensOptions = $derived(buildLensOptions(allLenses, selectedCamera, 'No lens selected', lensMounts));

	// Smart lens default: fixed > roll default > camera default (QuickAddBar seeds from this).
	const quickDefaultLensId = $derived.by(() => {
		if (fixedLens) return String(fixedLens.id);
		if (selectedRoll?.lens_id) return String(selectedRoll.lens_id);
		if (selectedCamera?.default_lens_id) return String(selectedCamera.default_lens_id);
		return '';
	});

	const frameCells = $derived(selectedRoll ? buildFrameCells(shots, selectedRoll.frame_count) : []);
	const nextFrameNumber = $derived(frameCells.find((c) => c.isNext)?.frameNumber ?? '');
	const frameToLog = $derived(jumpFrame || nextFrameNumber);

	// Frame progress for the roll-full nudge.
	const frameInfo = $derived.by(() => {
		if (!selectedRoll) return null;
		const total = selectedRoll.frame_count;
		return { current: shots.length, total: total ?? null };
	});

	// Roll-full nudge state
	let rollFullDismissed = $state(false);
	let finishDate = $state(todayLocal());
	const finishDateError = $derived(dateFieldError(finishDate));

	const showRollFullNudge = $derived(
		selectedRoll?.status === 'shooting' &&
			frameInfo !== null &&
			frameInfo.total !== null &&
			shots.length >= frameInfo.total &&
			!rollFullDismissed
	);

	async function markRollShot() {
		if (!selectedRoll || !finishDate.trim() || finishDateError) return;
		try {
			await updateRoll(selectedRoll.id, { status: 'shot', date_finished: finishDate });
			rolls = await listRolls();
			rollFullDismissed = true;
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	async function loadInitial() {
		try {
			const [r, cams, lenses, mounts] = await Promise.all([listRolls(), listCameras(), listLenses(), listLensMounts()]);
			rolls = r;
			cameras = cams;
			allLenses = lenses;
			lensMounts = mounts;

			// Pre-select roll from query param (e.g., /quick-entry?roll=42)
			const rollParam = page.url.searchParams.get('roll');
			if (rollParam && r.some((roll) => String(roll.id) === rollParam)) {
				selectedRollId = rollParam;
			}
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			loading = false;
		}
	}

	async function loadRollData(rollId: number) {
		error = '';
		try {
			shots = await listShotsForRoll(rollId);
			const roll = rolls.find((r) => r.id === rollId);
			finishDate = roll?.date_finished ?? todayLocal();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	async function handleQuickAdd(entry: {
		frameNumber: string;
		aperture: string;
		shutterSpeed: string;
		lensId: string;
		date: string;
		location: string;
		notes: string;
	}) {
		if (!selectedRoll || !entry.frameNumber.trim()) return;
		quickError = '';
		quickSaving = true;
		try {
			await logShot({ rollId: selectedRoll.id, ...entry });
			sessionCount++;
			jumpFrame = '';
			[shots, rolls] = await Promise.all([listShotsForRoll(selectedRoll.id), listRolls()]);
		} catch (err) {
			quickError = err instanceof Error ? err.message : String(err);
		} finally {
			quickSaving = false;
		}
	}

	// FrameStrip: open slot → target it next; filled frame → no-op (edit on roll page).
	function handleFrameSelect(frameNumber: string, shot: Shot | null) {
		if (shot === null) jumpFrame = frameNumber;
	}

	function handleAddExtra() {
		if (selectedRoll) jumpFrame = nextExtraFrameNumber(shots, selectedRoll.frame_count);
	}

	function changeRoll() {
		selectedRollId = '';
	}

	// Load a roll's shots when the selection changes. Only selectedRollId is tracked;
	// loadRollData's reactive reads happen after its first await, so this never loops.
	$effect(() => {
		if (selectedRollId) {
			rollFullDismissed = false;
			jumpFrame = '';
			loadRollData(Number(selectedRollId));
		} else {
			shots = [];
		}
	});

	$effect(() => {
		loadInitial();
	});
</script>

<PageHeader title="Quick Entry" description="Rapid shot logging — one frame at a time" />

<div class="p-6">
	{#if loading}
		<p class="text-sm text-text-muted">Loading...</p>
	{:else}
		{#if error}
			<div class="mb-4 rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
		{/if}

		{#if !selectedRoll}
			<!-- Roll picker: visual list of active rolls -->
			<h2 class="mb-3 flex items-center gap-3 text-xs font-semibold uppercase tracking-wider text-text-faint">
				Active rolls
				<div class="flex-1 border-b border-border-subtle"></div>
			</h2>
			{#if activeRolls.length === 0}
				<EmptyState title="No active rolls" message="Load a roll to start logging shots.">
					{#snippet icon()}<Film size={24} strokeWidth={1.5} />{/snippet}
					<Button variant="primary" href="/rolls/new">+ New Roll</Button>
				</EmptyState>
			{:else}
				<div class="grid gap-1.5">
					{#each activeRolls as roll, i (roll.id)}
						<FadeIn delay={Math.min(i, 10) * 30}>
							<RollRow {roll} onclick={() => (selectedRollId = String(roll.id))} />
						</FadeIn>
					{/each}
				</div>
			{/if}
		{:else}
			<!-- Collapsed selected roll + Change -->
			<div class="mb-5">
				<RollRow roll={selectedRoll} selected>
					{#snippet trailing()}
						<Button size="sm" variant="ghost" onclick={changeRoll}>Change</Button>
					{/snippet}
				</RollRow>
			</div>

			{#if showRollFullNudge}
				<FadeIn delay={50}>
					<div
						class="mb-5 flex flex-wrap items-end justify-between gap-3 rounded-lg border border-accent/30 bg-accent/10 px-4 py-3"
					>
						<div class="flex flex-col gap-2">
							<div>
								<p class="text-sm font-medium text-accent">Roll complete</p>
								<p class="text-xs text-accent/70">
									All {frameInfo?.total} frames shot. When did you finish it?
								</p>
							</div>
							<div class="w-44">
								<DateInput label="Finished shooting" bind:value={finishDate} />
							</div>
						</div>
						<div class="flex items-center gap-2">
							<Button
								size="sm"
								variant="primary"
								disabled={!finishDate.trim() || !!finishDateError}
								onclick={markRollShot}>Mark as Shot</Button
							>
							<button
								onclick={() => {
									rollFullDismissed = true;
								}}
								class="px-1 text-lg leading-none text-accent/60 transition-colors hover:text-accent"
								aria-label="Dismiss">&times;</button
							>
						</div>
					</div>
				</FadeIn>
			{/if}

			<!-- Entry bar + film strip -->
			<FadeIn delay={50}>
				<div class="space-y-2">
					<QuickAddBar
						frameNumber={frameToLog}
						{lensOptions}
						lensId={quickDefaultLensId}
						{isFixedLens}
						fixedLensLabel={fixedLens ? lensDisplayName(fixedLens) : ''}
						saving={quickSaving}
						error={quickError}
						onsave={handleQuickAdd}
					/>
					<FrameStrip frames={frameCells} onselect={handleFrameSelect} onaddextra={handleAddExtra} />
				</div>
			</FadeIn>

			{#if sessionCount > 0}
				<p class="mt-3 font-mono text-xs text-text-faint">{sessionCount} this session</p>
			{/if}
		{/if}
	{/if}
</div>
```

- [ ] **Step 2: Type-check and build**

Run: `cd frontend && bun run check`
Expected: PASS — no svelte-check errors. (Watch for: unused imports, missing `lens_id`/`frame_count` on `RollWithDetails` — both already exist on the type.)

- [ ] **Step 3: Commit**

```bash
git add "frontend/src/routes/(app)/quick-entry/+page.svelte"
git commit -m "feat(quick-entry): visual roll picker + shared film-strip logging (kammerz-ife)"
```

---

## Task 4: Full gate + manual verification

**Files:** none (verification only)

- [ ] **Step 1: Restore the build-wiped gitkeep**

The frontend build deletes `frontend/build/.gitkeep`. Restore it so its deletion isn't committed:

Run: `git checkout -- frontend/build/.gitkeep 2>/dev/null || true`

- [ ] **Step 2: Run the frontend unit tests**

Run: `cd frontend && bun run test:unit`
Expected: PASS — including the new `frames.test.ts` suite.

- [ ] **Step 3: Run the full local CI mirror**

Run: `just check`
Expected: PASS — `fmt-check`, `ci-backend`, `ci-frontend` (svelte-check + build + vitest) all green. If `fmt-check` fails, run `just fmt` then re-stage and amend the relevant commit.

- [ ] **Step 4: Manual smoke of Quick Entry**

Run `just dev`, open `http://localhost:5273/quick-entry`, and verify:

- Active rolls render as a stacked visual list (film-strip edge, badge, metadata, frame counter). Non-active rolls (archived/scanned/etc.) are absent.
- Clicking a roll collapses the list to one row with a **Change** button; clicking **Change** returns to the list.
- `QuickAddBar` logs a frame; the `FrameStrip` fills the slot and the next slot becomes highlighted; "N this session" increments.
- Clicking an **open** strip slot retargets the bar to that frame (the bar's Frame readout changes); saving there logs it, then logging returns to sequential. Clicking a **filled** slot does nothing.
- The **+** strip button targets the next over-roll frame (e.g. 37 on a 36 roll).
- A fixed-lens camera's roll shows the locked lens label in the bar.
- Visiting `http://localhost:5273/quick-entry?roll=<id>` preselects that roll.
- On a `shooting` roll with all frames shot, the "Roll complete" nudge appears and **Mark as Shot** works.

Stop `just dev` after confirming.

- [ ] **Step 5: Final commit if formatting changed anything**

```bash
git status   # if only formatting changed, amend; otherwise nothing to do
```

---

## Notes for the implementer

- **Beads hygiene:** This plan is feature work. Do NOT flip `kammerz-ife` to closed inside the PR branch — closing is a separate post-merge `chore(beads)` commit on `main`. If `.beads/issues.jsonl` shows up modified, `git restore --staged .beads/issues.jsonl` before committing.
- **Why `RollRow` uses `<svelte:element>`:** one component serves three element types — `<a>` (rolls list), `<button>` (picker select), `<div>` (collapsed row, so the inner Change `<button>` isn't an invalid nested button). The `interactive` derived gates the hover affordances to the clickable cases.
- **Why `QuickAddBar` needs no edit support here:** its Frame field is display-only (`{frameNumber || '—'}`) and it owns its own ⌘/Ctrl+Enter handler — the page only feeds it `frameToLog` and handles `onsave`. The old page's `handleKeydown` + `<svelte:window>` and the `frameNumber`/`aperture`/`shutterSpeed`/`notes`/`selectedLensId`/`suggestNextFrame` state are intentionally gone (the bar owns all of that now).

```
```
