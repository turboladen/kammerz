# Lifecycle Date Capture & Editing — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the user record the *real* date of every roll lifecycle milestone (including the missing Lab Done → "received back" date) via a confirm-on-transition prompt, and edit any date directly from an inline-editable Timeline.

**Architecture:** Frontend-only, reusing existing roll/lab/self update endpoints. The Timeline becomes the single home for dates: `buildRollTimeline` tags each milestone with a write `target` (roll/lab/self + field); a shared `DateConfirm` dialog is used both for inline edits (in a new `RollTimeline` component) and for the forward-transition prompt in `rolls/[id]/+page.svelte`. The roll Edit form drops its date pickers.

**Tech Stack:** SvelteKit, Svelte 5 runes (`$state`/`$derived`/`$props`/`$bindable`/`$effect`), Tailwind 4, TypeScript. No backend change.

**Testing note:** This project has **no JS unit-test runner** (only `svelte-check` + a tiny Playwright smoke suite). Per the established pattern, each task is verified with `bun run check` (types) and a targeted **browser** check via the Playwright MCP (run `just dev`, open `http://localhost:5273`), then a commit. `cargo test` must stay green (backend untouched). All `bun`/`cargo` commands run from the listed cwd.

---

## File Structure

**New**
- `frontend/src/lib/components/ui/DateConfirm.svelte` — shared date-pick dialog (Confirm / Cancel, optional Clear). Built on existing `Dialog` + `DateInput`.
- `frontend/src/lib/components/rolls/RollTimeline.svelte` — renders the milestone list and owns inline editing UI; emits `onedit(milestone, date|null)`.

**Modified**
- `frontend/src/lib/utils/timeline.ts` — add `DateTarget` + `target`/`editable` to milestones.
- `frontend/src/routes/(app)/rolls/[id]/+page.svelte` — `STATUS_DATE_TARGET`; refactor `updateStatus`; `handleStatusClick` opens `DateConfirm` on forward+empty; render `<RollTimeline onedit=…>`; remove the 5 edit-form `DateInput`s and dead `editDate*` state.
- `UI_DESIGN.md` — document the editable Timeline + `DateConfirm` pattern.

**Unchanged:** all Rust, `DevelopmentSection.svelte`, `$lib/api/development.ts` (reused: `updateLabDev`, `updateSelfDev`), `$lib/api/rolls.ts` (`updateRoll`).

---

## Task 1: Tag timeline milestones with a write target

**Files:**
- Modify: `frontend/src/lib/utils/timeline.ts` (whole file)

- [ ] **Step 1: Replace the file contents**

```ts
import type { RollWithDetails, DevelopmentLab, DevelopmentSelf } from '$lib/types';
import type { DevPath } from './status';

/** Where a milestone's date is stored, so the editor/prompt can write it back. */
export type DateTarget =
	| {
			kind: 'roll';
			field: 'date_loaded' | 'date_finished' | 'date_scanned' | 'date_post_processed' | 'date_archived';
	  }
	| { kind: 'lab'; field: 'date_dropped_off' | 'date_received' }
	| { kind: 'self'; field: 'date_processed' };

/** One dated milestone in a roll's lifecycle. `date` is null until the milestone is reached. */
export interface TimelineMilestone {
	key: string;
	label: string;
	date: string | null;
	/** Which record + column this date lives in (edit/transition write target). */
	target: DateTarget;
	/** False when the backing record doesn't exist yet (lab/self before a dev record). */
	editable: boolean;
}

/**
 * Build the ordered, path-aware lifecycle timeline for a roll.
 *
 * Dates come from heterogeneous sources: the roll row owns loaded / finished-shooting /
 * scanned / post-processed / archived, while the development middle is owned by the dev records
 * (lab: dropped-off + received-back; self: developed = date_processed). The lab/self middle is
 * mutually exclusive and mirrors labFlow/selfFlow in status.ts; the undecided path omits the
 * middle. A null date renders as a not-yet-reached milestone.
 */
export function buildRollTimeline(
	roll: RollWithDetails,
	labDev: DevelopmentLab | null,
	selfDev: DevelopmentSelf | null,
	devPath: DevPath
): TimelineMilestone[] {
	const milestones: TimelineMilestone[] = [
		{ key: 'loaded', label: 'Loaded', date: roll.date_loaded, target: { kind: 'roll', field: 'date_loaded' }, editable: true },
		{ key: 'finished-shooting', label: 'Finished shooting', date: roll.date_finished, target: { kind: 'roll', field: 'date_finished' }, editable: true }
	];

	if (devPath === 'lab') {
		milestones.push(
			{ key: 'dropped-off', label: 'Dropped off at lab', date: labDev?.date_dropped_off ?? null, target: { kind: 'lab', field: 'date_dropped_off' }, editable: labDev != null },
			{ key: 'received', label: 'Received back', date: labDev?.date_received ?? null, target: { kind: 'lab', field: 'date_received' }, editable: labDev != null }
		);
	} else if (devPath === 'self') {
		milestones.push({ key: 'developed', label: 'Developed', date: selfDev?.date_processed ?? null, target: { kind: 'self', field: 'date_processed' }, editable: selfDev != null });
	}

	milestones.push(
		{ key: 'scanned', label: 'Scanned', date: roll.date_scanned, target: { kind: 'roll', field: 'date_scanned' }, editable: true },
		{ key: 'post-processed', label: 'Post-processed', date: roll.date_post_processed, target: { kind: 'roll', field: 'date_post_processed' }, editable: true },
		{ key: 'archived', label: 'Archived', date: roll.date_archived, target: { kind: 'roll', field: 'date_archived' }, editable: true }
	);

	return milestones;
}
```

- [ ] **Step 2: Type-check**

Run (cwd `frontend`): `bun run check`
Expected: `0 ERRORS 0 WARNINGS` (the consumer in `[id]/+page.svelte` only reads `.date`/`.label`/`.key` so far, so it still compiles).

- [ ] **Step 3: Commit**

```bash
git add frontend/src/lib/utils/timeline.ts
git commit -m "feat(timeline): tag lifecycle milestones with a date write target (kammerz-b08)"
```

---

## Task 2: `DateConfirm` shared date-pick dialog

**Files:**
- Create: `frontend/src/lib/components/ui/DateConfirm.svelte`

Reference APIs (already in the codebase):
- `Dialog.svelte` props: `open` (bindable), `title`, `children`, `onclose?`.
- `DateInput.svelte` props: `label?`, `hint?`, `value` (bindable string).
- `Button.svelte` variants: `primary`, `ghost`.

- [ ] **Step 1: Create the component**

```svelte
<script lang="ts">
	// Small date-pick dialog used for both inline Timeline edits and the
	// confirm-on-transition prompt. Seeds an editable date (default today from the
	// caller), Confirm commits it, Cancel aborts. Inline edits also offer Clear
	// (commit null). There is no "Skip" — callers that want a blank date clear it
	// from the Timeline afterward.
	import Dialog from './Dialog.svelte';
	import DateInput from './DateInput.svelte';
	import Button from './Button.svelte';

	interface Props {
		open: boolean;
		title: string;
		/** Seed value; caller passes today for transition prompts, the current date for edits. */
		value?: string;
		confirmLabel?: string;
		/** Show a Clear button (commits null) — used for inline edits, not transitions. */
		allowClear?: boolean;
		onconfirm: (date: string | null) => void;
		oncancel: () => void;
	}

	let {
		open = $bindable(),
		title,
		value = '',
		confirmLabel = 'Confirm',
		allowClear = false,
		onconfirm,
		oncancel
	}: Props = $props();

	let draft = $state(value);
	// Re-seed each time the dialog opens so a reused instance starts fresh.
	$effect(() => {
		if (open) draft = value;
	});

	function confirm() {
		onconfirm(draft.trim() ? draft.trim() : null);
	}
</script>

<Dialog bind:open {title} onclose={oncancel}>
	<div class="space-y-4">
		<DateInput label="Date" bind:value={draft} />
		<div class="flex justify-end gap-2">
			{#if allowClear}
				<Button variant="ghost" onclick={() => onconfirm(null)}>Clear</Button>
			{/if}
			<Button variant="ghost" onclick={oncancel}>Cancel</Button>
			<Button variant="primary" onclick={confirm}>{confirmLabel}</Button>
		</div>
	</div>
</Dialog>
```

- [ ] **Step 2: Type-check**

Run (cwd `frontend`): `bun run check`
Expected: `0 ERRORS 0 WARNINGS`.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/lib/components/ui/DateConfirm.svelte
git commit -m "feat(ui): DateConfirm date-pick dialog (kammerz-b08)"
```

---

## Task 3: `RollTimeline` component with inline editing

**Files:**
- Create: `frontend/src/lib/components/rolls/RollTimeline.svelte`

This reproduces the current Timeline markup (`[id]/+page.svelte:799-812`: dot + label + dashed rule + mono date) and adds a per-row edit affordance that opens `DateConfirm`.

- [ ] **Step 1: Create the component**

```svelte
<script lang="ts">
	// Inline-editable lifecycle timeline. Renders milestones (dot + label + dashed
	// rule + date) and lets the user set/change/clear each editable date via a
	// DateConfirm dialog. Emits onedit(milestone, date|null); the parent routes the
	// write to the roll/lab/self record. Pure UI — no data fetching here.
	import { Pencil } from 'lucide-svelte';
	import DateConfirm from '$lib/components/ui/DateConfirm.svelte';
	import { todayLocal } from '$lib/utils/date';
	import type { TimelineMilestone } from '$lib/utils/timeline';

	interface Props {
		milestones: TimelineMilestone[];
		onedit: (milestone: TimelineMilestone, date: string | null) => void;
	}

	let { milestones, onedit }: Props = $props();

	let editOpen = $state(false);
	let editing: TimelineMilestone | null = $state(null);

	function startEdit(m: TimelineMilestone) {
		editing = m;
		editOpen = true;
	}
	function confirmEdit(date: string | null) {
		if (editing) onedit(editing, date);
		editOpen = false;
		editing = null;
	}
	function cancelEdit() {
		editOpen = false;
		editing = null;
	}
</script>

<ol class="space-y-1.5">
	{#each milestones as milestone (milestone.key)}
		<li class="flex items-center gap-3 text-sm">
			<span class="h-1.5 w-1.5 shrink-0 rounded-full {milestone.date ? 'bg-accent' : 'bg-surface-overlay'}"></span>
			<span class={milestone.date ? 'text-text-muted' : 'text-text-faint'}>{milestone.label}</span>
			<div class="flex-1 border-b border-dashed border-border-subtle/60"></div>
			{#if milestone.editable}
				<button
					onclick={() => startEdit(milestone)}
					class="group flex items-center gap-1.5 rounded px-1 py-0.5 transition-colors hover:bg-surface-overlay"
					aria-label={milestone.date ? `Edit ${milestone.label} date` : `Set ${milestone.label} date`}
				>
					<span class="font-mono text-xs {milestone.date ? 'text-text' : 'text-text-faint'}">
						{milestone.date ?? 'Set date'}
					</span>
					<Pencil size={12} strokeWidth={1.75} class="text-text-faint opacity-0 transition-opacity group-hover:opacity-100" />
				</button>
			{:else}
				<span class="font-mono text-xs text-text-faint">{milestone.date ?? '—'}</span>
			{/if}
		</li>
	{/each}
</ol>

{#if editing}
	<DateConfirm
		bind:open={editOpen}
		title={editing.date ? `Edit “${editing.label}” date` : `Set “${editing.label}” date`}
		value={editing.date ?? todayLocal()}
		confirmLabel="Save"
		allowClear={!!editing.date}
		onconfirm={confirmEdit}
		oncancel={cancelEdit}
	/>
{/if}
```

- [ ] **Step 2: Type-check**

Run (cwd `frontend`): `bun run check`
Expected: `0 ERRORS 0 WARNINGS`. (Confirms `todayLocal` is exported from `$lib/utils/date` — it is; the page already imports it.)

- [ ] **Step 3: Commit**

```bash
git add frontend/src/lib/components/rolls/RollTimeline.svelte
git commit -m "feat(rolls): RollTimeline component with inline date editing (kammerz-b08)"
```

---

## Task 4: Render `RollTimeline` and route date edits

**Files:**
- Modify: `frontend/src/routes/(app)/rolls/[id]/+page.svelte` (imports; timeline render ~792-814; add a save handler)

- [ ] **Step 1: Add imports**

Find the existing UI imports near the top (e.g. the `import FadeIn from '$lib/components/ui/FadeIn.svelte';` line) and add below them:

```ts
	import RollTimeline from '$lib/components/rolls/RollTimeline.svelte';
	import { updateLabDev, updateSelfDev } from '$lib/api/development';
	import type { TimelineMilestone } from '$lib/utils/timeline';
```

(`updateRoll` is already imported; `labDev`/`selfDev` are already `$state` in this file.)

- [ ] **Step 2: Add the date-edit router**

Place this near `updateStatus` (anywhere in the `<script>`). It writes a milestone's new date (or null to clear) to the correct record, then refreshes:

```ts
	// Persist an inline Timeline date edit to whichever record owns it, then refresh.
	async function saveTimelineDate(milestone: TimelineMilestone, date: string | null) {
		error = '';
		try {
			const t = milestone.target;
			if (t.kind === 'roll') {
				await updateRoll(id, { [t.field]: date });
			} else if (t.kind === 'lab' && labDev) {
				await updateLabDev(labDev.id, { [t.field]: date });
			} else if (t.kind === 'self' && selfDev) {
				await updateSelfDev(selfDev.id, { [t.field]: date });
			}
			await loadRollData();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}
```

- [ ] **Step 3: Replace the inline timeline markup**

Replace the `<ol class="space-y-1.5"> … </ol>` block (currently `[id]/+page.svelte:799-812`) inside the `<!-- Lifecycle Timeline -->` section with:

```svelte
				<RollTimeline milestones={timeline} onedit={saveTimelineDate} />
```

(The surrounding `<FadeIn delay={75}>` wrapper, the `<h2>Timeline</h2>` header, and the `<div class="mb-6">` stay.)

- [ ] **Step 4: Type-check**

Run (cwd `frontend`): `bun run check`
Expected: `0 ERRORS 0 WARNINGS`.

- [ ] **Step 5: Browser-verify (Playwright MCP)**

Start dev if not running: `just dev` (from repo root, background). Open `http://localhost:5273`, go to a roll on the **lab** path (status at-lab/lab-done so lab milestones show). Verify:
- Timeline renders as before, each editable row shows the date (or "Set date") with a pencil on hover.
- Click "Received back" → DateConfirm opens → pick a date → Save → the row now shows it (and the dot turns amber). Confirm the roll's `date_received` persisted via `sqlite3 kammerz.db "SELECT date_received FROM development_lab WHERE roll_id=<id>;"`.
- Click an existing date → Clear → it reverts to "Set date" / dot greys.
- Reload: edits persist.

- [ ] **Step 6: Commit**

```bash
git add "frontend/src/routes/(app)/rolls/[id]/+page.svelte"
git commit -m "feat(rolls): inline-editable Timeline via RollTimeline (kammerz-b08)"
```

---

## Task 5: Confirm-on-transition (generalize the date prompt)

**Files:**
- Modify: `frontend/src/routes/(app)/rolls/[id]/+page.svelte` (`STATUS_DATE_FIELD`→`STATUS_DATE_TARGET` ~103-110; `updateStatus` ~471-507; `handleStatusClick` ~509-518; add `DateConfirm` instance + state; update the backward-confirm and roll-full-nudge call sites)

Behavior: a **forward** advance into a status with a date target whose target date is **empty** opens `DateConfirm` (default today). **Confirm** advances status *and* writes the date to the target record; **Cancel** changes nothing. At Lab / Developing have **no** target (their full dialogs still auto-open). Backward moves keep `ConfirmDialog` and never touch dates.

- [ ] **Step 1: Replace `STATUS_DATE_FIELD` with `STATUS_DATE_TARGET`**

Replace the existing block (`[id]/+page.svelte:103-110`):

```ts
	const STATUS_DATE_FIELD: Partial<
		Record<RollStatus, 'date_scanned' | 'date_post_processed' | 'date_archived'>
	> = {
		scanned: 'date_scanned',
		'post-processed': 'date_post_processed',
		archived: 'date_archived'
	};
```

with (uses the `DateTarget` type — add it to the existing `timeline` import or import separately; Task 4 already imported `TimelineMilestone`, so extend that import to also bring `DateTarget`):

```ts
	// Which date a forward transition into each status records, and where it lives.
	// At-lab / developing are intentionally absent — their full dev dialogs capture
	// those dates. Shot records date_finished (the roll-full nudge also sets it).
	const STATUS_DATE_TARGET: Partial<Record<RollStatus, DateTarget>> = {
		shot: { kind: 'roll', field: 'date_finished' },
		'lab-done': { kind: 'lab', field: 'date_received' },
		developed: { kind: 'self', field: 'date_processed' },
		scanned: { kind: 'roll', field: 'date_scanned' },
		'post-processed': { kind: 'roll', field: 'date_post_processed' },
		archived: { kind: 'roll', field: 'date_archived' }
	};

	// Helper: the current value of a status's target date (for the forward+empty check).
	function targetDate(t: DateTarget): string | null {
		if (!roll) return null;
		if (t.kind === 'roll') return roll[t.field] ?? null;
		if (t.kind === 'lab') return labDev?.[t.field] ?? null;
		return selfDev?.[t.field] ?? null;
	}
```

Update the import added in Task 4 to:

```ts
	import type { TimelineMilestone, DateTarget } from '$lib/utils/timeline';
```

- [ ] **Step 2: Add the transition-prompt state**

Add near the other `$state` declarations:

```ts
	// Confirm-on-transition prompt state.
	let datePromptOpen = $state(false);
	let datePromptStatus: RollStatus | null = $state(null);
	let datePromptLabel = $state('');
```

- [ ] **Step 3: Rewrite `updateStatus` to take an explicit date**

Replace the whole `updateStatus` function (`[id]/+page.svelte:471-507`) with:

```ts
	// Commit a status change. When `date` is provided (from the prompt or the
	// roll-full nudge) and the move is a forward advance, also write it to the
	// status's target record. Backward moves never write a date.
	async function updateStatus(status: RollStatus, date?: string) {
		error = '';
		try {
			const patch: Partial<RollInsert> = { status };
			const target = STATUS_DATE_TARGET[status];
			const targetIdx = statusFlow.indexOf(status);
			const advancing = currentStatusIdx === -1 || targetIdx > currentStatusIdx;

			// Roll-owned dates go in the same PATCH as the status. Use Object.assign
			// with a computed key (not `patch[target.field] = date`) — a union-keyed
			// index assignment trips TS2322 ("not assignable to never").
			if (advancing && date && target?.kind === 'roll') {
				Object.assign(patch, { [target.field]: date });
			}
			await updateRoll(id, patch);

			// Dev-owned dates (lab/self) are a follow-up write to the dev record.
			if (advancing && date && target?.kind === 'lab' && labDev) {
				await updateLabDev(labDev.id, { [target.field]: date });
			} else if (advancing && date && target?.kind === 'self' && selfDev) {
				await updateSelfDev(selfDev.id, { [target.field]: date });
			}

			await loadRollData();

			// Auto-prompt development dialogs (unchanged).
			if (status === 'at-lab' && !labDev && !selfDev) {
				devAutoPrompt = 'lab';
			} else if (status === 'developing' && !selfDev && !labDev) {
				devAutoPrompt = 'self';
			}
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}
```

- [ ] **Step 4: Make `handleStatusClick` open the prompt on forward+empty**

Replace `handleStatusClick` (`[id]/+page.svelte:509-518`) with:

```ts
	function handleStatusClick(status: RollStatus) {
		if (!roll) return;
		const targetIdx = statusFlow.indexOf(status);
		// Backward move — confirm, never touch dates.
		if (currentStatusIdx !== -1 && targetIdx < currentStatusIdx) {
			pendingStatus = status;
			return;
		}
		// Forward into a date-bearing status whose date isn't recorded yet → prompt.
		const target = STATUS_DATE_TARGET[status];
		const recordExists = !target || target.kind === 'roll' || (target.kind === 'lab' && labDev) || (target.kind === 'self' && selfDev);
		if (target && recordExists && !targetDate(target)) {
			datePromptStatus = status;
			datePromptLabel = statusConfig[status].label;
			datePromptOpen = true;
			return;
		}
		// Otherwise advance directly (no date to capture, or already recorded).
		updateStatus(status);
	}

	function confirmDatePrompt(date: string | null) {
		const status = datePromptStatus;
		datePromptOpen = false;
		datePromptStatus = null;
		if (status) updateStatus(status, date ?? undefined);
	}

	function cancelDatePrompt() {
		datePromptOpen = false;
		datePromptStatus = null;
	}
```

- [ ] **Step 5: Update the backward-confirm call site**

The backward-move `ConfirmDialog`'s confirm action currently calls `updateStatus(pendingStatus)`. That signature is unchanged (date omitted → status only), so **no edit needed** — but verify the call still reads `updateStatus(pendingStatus!)` / `updateStatus(pendingStatus)` and passes no date. (Grep: `grep -n "updateStatus(pendingStatus" "frontend/src/routes/(app)/rolls/[id]/+page.svelte"`.)

- [ ] **Step 6: Update the roll-full nudge call site**

Find the nudge's "mark complete" handler (it calls `updateStatus('shot', finishDate)`; grep `updateStatus('shot'`). The new signature is `updateStatus(status, date?)`, so `updateStatus('shot', finishDate)` still works unchanged — confirm it compiles (the old `finishDateOverride` param is now `date`). No behavior change: the nudge passes its editable `finishDate`.

- [ ] **Step 7: Add the `DateConfirm` instance for transitions**

Near the end of the template (e.g. beside the existing backward-move `ConfirmDialog`), add:

```svelte
<DateConfirm
	bind:open={datePromptOpen}
	title={`Date for “${datePromptLabel}”`}
	value={todayLocal()}
	confirmLabel="Confirm"
	onconfirm={confirmDatePrompt}
	oncancel={cancelDatePrompt}
/>
```

Add the import near the other UI imports:

```ts
	import DateConfirm from '$lib/components/ui/DateConfirm.svelte';
```

- [ ] **Step 8: Type-check**

Run (cwd `frontend`): `bun run check`
Expected: `0 ERRORS 0 WARNINGS`.

- [ ] **Step 9: Browser-verify (Playwright MCP)**

On a roll on the lab path:
- Click the **Lab Done** chevron (forward, `date_received` empty) → DateConfirm opens defaulting to today → back-date it → Confirm → status advances to Lab Done **and** the Timeline "Received back" shows the back-dated date. Verify `sqlite3 kammerz.db "SELECT date_received FROM development_lab WHERE roll_id=<id>;"`.
- Click **Scanned** → prompt → Confirm → `date_scanned` set; click **Cancel** on the next (Post-processed) prompt → status does **not** change.
- Click **At Lab** on a fresh lab roll → the full Lab dialog still auto-opens (no double prompt).
- Move a status **backward** → the existing ConfirmDialog appears; confirm → status reverts, dates untouched.

- [ ] **Step 10: Commit**

```bash
git add "frontend/src/routes/(app)/rolls/[id]/+page.svelte"
git commit -m "feat(rolls): confirm-on-transition date prompt incl. Lab Done (kammerz-b08)"
```

---

## Task 6: Remove date pickers from the roll Edit form

**Files:**
- Modify: `frontend/src/routes/(app)/rolls/[id]/+page.svelte` (state ~55-59; `startEditRoll` seeds ~544-548; `saveEditRoll` patch ~564-568; edit-form markup ~646-654)

Dates now live solely in the Timeline; the Edit form keeps Fuzzy Date and all non-date fields.

- [ ] **Step 1: Remove the dead edit-date state**

Delete these five lines (`[id]/+page.svelte:55-59`):

```ts
	let editDateLoaded = $state('');
	let editDateFinished = $state('');
	let editDateScanned = $state('');
	let editDatePostProcessed = $state('');
	let editDateArchived = $state('');
```

(Keep `let editDateFuzzy = $state('');`.)

- [ ] **Step 2: Remove their seeds in `startEditRoll`**

Delete these lines (`~544-548`):

```ts
		editDateLoaded = roll.date_loaded ?? '';
		editDateFinished = roll.date_finished ?? '';
		editDateScanned = roll.date_scanned ?? '';
		editDatePostProcessed = roll.date_post_processed ?? '';
		editDateArchived = roll.date_archived ?? '';
```

(Keep `editDateFuzzy = roll.date_fuzzy ?? '';`.)

- [ ] **Step 3: Remove their fields from the `saveEditRoll` patch**

Delete these lines (`~564-568`):

```ts
				date_loaded: editDateLoaded || null,
				date_finished: editDateFinished || null,
				date_scanned: editDateScanned || null,
				date_post_processed: editDatePostProcessed || null,
				date_archived: editDateArchived || null,
```

(Keep `date_fuzzy: editDateFuzzy || null,` and all non-date fields. The Edit form no longer writes these dates — the Timeline owns them.)

- [ ] **Step 4: Remove the date inputs from the edit-form markup**

Delete the two grids (`~646-654`):

```svelte
					<div class="grid grid-cols-2 gap-4">
						<DateInput label="Date Loaded" bind:value={editDateLoaded} />
						<DateInput label="Finished Shooting" bind:value={editDateFinished} />
					</div>
					<div class="grid grid-cols-3 gap-4">
						<DateInput label="Scanned" bind:value={editDateScanned} />
						<DateInput label="Post-processed" bind:value={editDatePostProcessed} />
						<DateInput label="Archived" bind:value={editDateArchived} />
					</div>
```

(Keep the Push/Pull + Fuzzy Date grid and the `<DateInput>` import — it's still used by `DateConfirm`/`RollTimeline` indirectly, and `editDateFuzzy` uses an `<Input>`. If `bun run check` flags `DateInput` as unused in this file, remove its import here.)

- [ ] **Step 5: Type-check**

Run (cwd `frontend`): `bun run check`
Expected: `0 ERRORS 0 WARNINGS`. Resolve any "unused" warning by removing the now-dead import/variable it names.

- [ ] **Step 6: Browser-verify**

Open a roll → **Edit** → confirm the form has **no** date pickers (only roll ID, camera, film, lens, frame count, push/pull, fuzzy date, notes) and saves fine; dates are still editable from the Timeline.

- [ ] **Step 7: Commit**

```bash
git add "frontend/src/routes/(app)/rolls/[id]/+page.svelte"
git commit -m "refactor(rolls): move lifecycle dates out of the Edit form into the Timeline (kammerz-b08)"
```

---

## Task 7: Final verification + docs

**Files:**
- Modify: `UI_DESIGN.md`

- [ ] **Step 1: Backend tests still green**

Run (cwd repo root): `cargo test -p kammerz`
Expected: all pass (backend untouched).

- [ ] **Step 2: Frontend gates**

Run (cwd `frontend`): `bun run check` then `bun run build`
Expected: `0 ERRORS 0 WARNINGS`; build `✔ done`.

- [ ] **Step 3: Full browser walk (Playwright MCP)**

`just dev` → `http://localhost:5273`. Walk a roll through the **lab** path end to end: Shot prompt → At Lab dialog → **Lab Done prompt (back-dated)** → Scanned/Post-processed/Archived prompts (try Cancel on one) → inline-edit and **Clear** a Timeline date → confirm status is unaffected by date edits, the Edit form has no date pickers, and a reload shows everything persisted. Spot-check the DB: `sqlite3 kammerz.db "SELECT date_finished,date_scanned,date_post_processed,date_archived FROM rolls WHERE id=<id>; SELECT date_dropped_off,date_received FROM development_lab WHERE roll_id=<id>;"`.

- [ ] **Step 4: Document the pattern in `UI_DESIGN.md`**

Under the Components section (near the `EmptyState`/`FadeIn` entries), add:

```markdown
### DateConfirm (`frontend/src/lib/components/ui/DateConfirm.svelte`)

Small date-pick dialog (built on `Dialog` + `DateInput`). Confirm / Cancel, with an
optional **Clear** (commits null) for inline edits. Used for both the
confirm-on-transition prompt and inline Timeline date editing. No "Skip" — to leave a
date blank, advance then Clear it in the Timeline.

### RollTimeline (`frontend/src/lib/components/rolls/RollTimeline.svelte`)

The roll lifecycle Timeline (dot + label + dashed rule + date), with each editable
milestone click-to-edit via `DateConfirm`. Emits `onedit(milestone, date|null)`; the
roll-detail page routes the write to the roll / lab-dev / self-dev record by the
milestone's `target` (see `buildRollTimeline` in `utils/timeline.ts`). The Timeline is
the single home for lifecycle dates — the roll Edit form has no date pickers.
```

- [ ] **Step 5: Commit**

```bash
git add UI_DESIGN.md
git commit -m "docs(ui): document DateConfirm + editable RollTimeline (kammerz-b08)"
```

---

## Notes for the implementer

- **Svelte 5 index access on `$state` records:** *reading* `roll[t.field]`, `labDev?.[t.field]` is fine (the `t.kind` narrowing gives a string-literal field union matching the model). For *writing*, never assign through a union-keyed index (`patch[target.field] = …` → TS2322); always build an object literal with a computed key (`Object.assign(patch, { [target.field]: date })` or `updateRoll(id, { [t.field]: date })`), as the tasks do.
- **`updateLabDev`/`updateSelfDev` payloads:** both take `Partial<…Insert>`, so `{ [t.field]: date }` (string or null) is valid. Clearing sends `null`.
- **Don't touch the backend or `DevelopmentSection.svelte`.** The At Lab / Developing dialogs still own dropoff/processed capture at creation time; the new prompt only fills the gaps and the silent stamps.
- **Status safety:** never call `updateStatus` from `saveTimelineDate` — editing a date must not change status.
```
