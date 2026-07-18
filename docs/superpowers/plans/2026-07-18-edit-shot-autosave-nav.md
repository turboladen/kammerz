# Edit Shot Auto-Save-on-Navigate (kammerz-11o3) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Navigating between shots with the Edit Shot dialog's < > arrows (buttons or arrow keys) auto-saves the current shot's edits instead of silently discarding them.

**Architecture:** Extract the shot-form payload/dirty-compare logic into a pure util (`shot-form.ts`, unit-tested with vitest), then rewire the dialog's prev/next navigation in `rolls/[id]/+page.svelte` to save-if-dirty before loading the adjacent shot. An e2e regression test in the Playwright smoke suite proves edits persist across navigation.

**Tech Stack:** Svelte 5 runes, vitest (`frontend/src/lib/utils/*.test.ts` co-located pattern), Playwright (`frontend/tests/smoke.spec.ts`).

## Global Constraints

- Bead: kammerz-11o3. Work on branch `kammerz-11o3-edit-shot-autosave-nav`; PRs are squash-merged.
- Do NOT commit `.beads/**` changes into this feature branch (`git restore --staged .beads/issues.jsonl` if it shows up); bead close happens post-merge on `main`.
- Aperture/shutter are stored BARE (`2.8`, `1/250`) — `normalizeAperture`/`normalizeShutter` already enforce this; never add an `f/` prefix or `s` suffix to stored values.
- Time payload rule (must match existing `shotTimePayload` behavior): valid → canonical `HH:MM`; blank/whitespace → `null`; invalid → trimmed raw string (so the backend 422s visibly).
- `bun run build` wipes `frontend/build/.gitkeep` — restore with `git checkout -- frontend/build/.gitkeep` before committing.
- Run `just fmt` before each commit; `just ci` before opening the PR.

---

### Task 1: `shot-form.ts` util (payload builder + dirty compare)

**Files:**

- Create: `frontend/src/lib/utils/shot-form.ts`
- Test: `frontend/src/lib/utils/shot-form.test.ts`

**Interfaces:**

- Consumes: `normalizeAperture`/`normalizeShutter` from `$lib/utils/exposure`, `parseTime` from `$lib/utils/time` (all existing).
- Produces (Task 2 depends on these exact names):
  - `interface ShotFormFields { frameNumber: string; aperture: string; shutterSpeed: string; date: string; time: string; location: string; notes: string; lensId: string }`
  - `shotFormsEqual(a: ShotFormFields, b: ShotFormFields): boolean`
  - `buildShotUpdatePayload(f: ShotFormFields): { frame_number: string; aperture: string | null; shutter_speed: string | null; date: string | null; time: string | null; location: string | null; notes: string | null; lens_ids: number[] }`

- [ ] **Step 1: Write the failing test**

```typescript
// frontend/src/lib/utils/shot-form.test.ts
import { describe, expect, it } from 'vitest';
import { buildShotUpdatePayload, shotFormsEqual, type ShotFormFields } from './shot-form';

const base: ShotFormFields = {
	frameNumber: '9',
	aperture: '5.6',
	shutterSpeed: '1/250',
	date: '2024-06-16',
	time: '12:30',
	location: 'Corlieu Falls',
	notes: 'first light',
	lensId: '3'
};

describe('shotFormsEqual', () => {
	it('is true for identical field sets', () => {
		expect(shotFormsEqual(base, { ...base })).toBe(true);
	});

	it('is false when any single field differs', () => {
		for (const key of Object.keys(base) as (keyof ShotFormFields)[]) {
			expect(shotFormsEqual(base, { ...base, [key]: base[key] + 'x' })).toBe(false);
		}
	});
});

describe('buildShotUpdatePayload', () => {
	it('maps populated fields, normalizing exposure values to bare form', () => {
		expect(buildShotUpdatePayload({ ...base, frameNumber: ' 9 ', aperture: 'f/2.8', shutterSpeed: '1/250s' })).toEqual({
			frame_number: '9',
			aperture: '2.8',
			shutter_speed: '1/250',
			date: '2024-06-16',
			time: '12:30',
			location: 'Corlieu Falls',
			notes: 'first light',
			lens_ids: [3]
		});
	});

	it('collapses empty optionals to null and empty lens to []', () => {
		expect(
			buildShotUpdatePayload({
				frameNumber: '1',
				aperture: '',
				shutterSpeed: '',
				date: '',
				time: '   ',
				location: '',
				notes: '',
				lensId: ''
			})
		).toEqual({
			frame_number: '1',
			aperture: null,
			shutter_speed: null,
			date: null,
			time: null,
			location: null,
			notes: null,
			lens_ids: []
		});
	});

	it('canonicalizes a valid compact time and passes an invalid time through raw', () => {
		expect(buildShotUpdatePayload({ ...base, time: '1430' }).time).toBe('14:30');
		expect(buildShotUpdatePayload({ ...base, time: '99:99' }).time).toBe('99:99');
	});
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd frontend && bun run test:unit shot-form`
Expected: FAIL — `Cannot find module './shot-form'` (or equivalent resolve error).

- [ ] **Step 3: Write the implementation**

```typescript
// frontend/src/lib/utils/shot-form.ts
import { normalizeAperture, normalizeShutter } from '$lib/utils/exposure';
import { parseTime } from '$lib/utils/time';

/**
 * The Edit/Add Shot dialog's form fields as plain strings — the shape the
 * roll-detail page holds in $state. Pure so both the dirty-compare and the
 * update payload can be unit tested away from the component (kammerz-11o3).
 */
export interface ShotFormFields {
	frameNumber: string;
	aperture: string;
	shutterSpeed: string;
	date: string;
	time: string;
	location: string;
	notes: string;
	lensId: string;
}

/** Field-by-field equality — used to decide whether navigation must save first. */
export function shotFormsEqual(a: ShotFormFields, b: ShotFormFields): boolean {
	return (
		a.frameNumber === b.frameNumber &&
		a.aperture === b.aperture &&
		a.shutterSpeed === b.shutterSpeed &&
		a.date === b.date &&
		a.time === b.time &&
		a.location === b.location &&
		a.notes === b.notes &&
		a.lensId === b.lensId
	);
}

/**
 * Build the PUT /api/shots/{id} body from the form fields. Exposure values are
 * normalized to their bare stored form; a valid time canonicalizes to HH:MM,
 * blank collapses to null, and an invalid time passes through raw so the
 * backend's 422 surfaces the mistake instead of silently dropping it.
 */
export function buildShotUpdatePayload(f: ShotFormFields) {
	return {
		frame_number: f.frameNumber.trim(),
		aperture: normalizeAperture(f.aperture) || null,
		shutter_speed: normalizeShutter(f.shutterSpeed) || null,
		date: f.date || null,
		time: parseTime(f.time) || f.time.trim() || null,
		location: f.location || null,
		notes: f.notes || null,
		lens_ids: f.lensId ? [Number(f.lensId)] : []
	};
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd frontend && bun run test:unit shot-form`
Expected: PASS (3 describe blocks, all green).

- [ ] **Step 5: Commit**

```bash
just fmt
git add frontend/src/lib/utils/shot-form.ts frontend/src/lib/utils/shot-form.test.ts
git commit -m "feat(shots): extract shot-form payload/dirty-compare util (kammerz-11o3)"
```

---

### Task 2: Auto-save-on-navigate wiring in the roll detail page

**Files:**

- Modify: `frontend/src/routes/(app)/rolls/[id]/+page.svelte` (shot form state ~line 124, `openEditShotDialog`/`goPrevShot`/`goNextShot` ~lines 525–559, `handleSaveShot` ~lines 561–601, nav buttons in the dialog markup ~lines 1290–1313)

**Interfaces:**

- Consumes: `ShotFormFields`, `shotFormsEqual`, `buildShotUpdatePayload` from `$lib/utils/shot-form` (Task 1).
- Produces: no new exports — behavior change only. `goPrevShot`/`goNextShot` keep their names (the keyboard handler and buttons already call them) but become async save-then-navigate.

- [ ] **Step 1: Add imports, snapshot/saving state, and a fields helper**

In the `<script>` block, add to the imports:

```typescript
import { buildShotUpdatePayload, shotFormsEqual, type ShotFormFields } from '$lib/utils/shot-form';
```

Near the shot form fields (after `let shotError = $state('');`), add:

```typescript
// Snapshot of the form as loaded when the Edit dialog opened — navigation
// compares against it to decide whether it must save first (kammerz-11o3).
let shotOpenSnapshot: ShotFormFields | null = $state(null);
// Guards double-fired navigation while a nav-triggered save is in flight.
let shotNavSaving = $state(false);

function currentShotFormFields(): ShotFormFields {
	return {
		frameNumber: shotFrameNumber,
		aperture: shotAperture,
		shutterSpeed: shotShutterSpeed,
		date: shotDate,
		time: shotTime,
		location: shotLocation,
		notes: shotNotes,
		lensId: shotLensId
	};
}
```

- [ ] **Step 2: Capture the snapshot on open, clear it on reset**

At the end of `openEditShotDialog` (before `showShotDialog = true;`), add:

```typescript
shotOpenSnapshot = currentShotFormFields();
```

In `resetShotForm()`, add as the last line:

```typescript
shotOpenSnapshot = null;
```

- [ ] **Step 3: Replace the navigation functions with save-then-navigate**

Replace the existing `goPrevShot`/`goNextShot`:

```typescript
function goPrevShot() {
	if (hasPrevShot) openEditShotDialog(orderedShots[currentShotIdx - 1]);
}
function goNextShot() {
	if (hasNextShot) openEditShotDialog(orderedShots[currentShotIdx + 1]);
}
```

with:

```typescript
// Navigate to an adjacent shot, saving the current one's edits first if the
// form is dirty (kammerz-11o3). The target is captured by POSITION before the
// save/reload (that's the shot the user saw next to the arrow), then re-looked
// up BY ID afterward — a frame-number edit can reorder orderedShots.
async function navigateToShot(direction: -1 | 1) {
	if (shotNavSaving || editingShotId == null) return;
	const target = orderedShots[currentShotIdx + direction];
	if (!target) return;
	const fields = currentShotFormFields();
	if (shotOpenSnapshot == null || !shotFormsEqual(fields, shotOpenSnapshot)) {
		shotError = '';
		if (!fields.frameNumber.trim()) {
			shotError = 'Frame number is required.';
			return;
		}
		if (shotDateError) return; // arrows are disabled too — belt and suspenders
		shotNavSaving = true;
		try {
			await updateShot(editingShotId, buildShotUpdatePayload(fields));
			await loadRollData('shot-add');
		} catch (err) {
			shotError = err instanceof Error ? err.message : String(err);
			return; // stay on this shot — the user can see and fix the failure
		} finally {
			shotNavSaving = false;
		}
	}
	const fresh = orderedShots.find((s) => s.id === target.id);
	if (fresh) openEditShotDialog(fresh);
}

function goPrevShot() {
	if (hasPrevShot) void navigateToShot(-1);
}
function goNextShot() {
	if (hasNextShot) void navigateToShot(1);
}
```

(The `'shot-add'` reload reason matches what `handleSaveShot` already uses for the edit path — it only affects the auto-status-notice wording, and an edit doesn't move status.)

- [ ] **Step 4: DRY `handleSaveShot`'s edit branch through the util**

In `handleSaveShot`, replace the edit-branch payload literal:

```typescript
if (editingShotId) {
	await updateShot(editingShotId, {
		frame_number: shotFrameNumber.trim(),
		aperture: normalizeAperture(shotAperture) || null,
		shutter_speed: normalizeShutter(shotShutterSpeed) || null,
		date: shotDate || null,
		time: shotTimePayload,
		location: shotLocation || null,
		notes: shotNotes || null,
		lens_ids: lensIds
	});
}
```

with:

```typescript
if (editingShotId) {
	await updateShot(editingShotId, buildShotUpdatePayload(currentShotFormFields()));
}
```

Leave the `createShot` branches (in `handleSaveShot` and `handleSaveShotAndNext`) untouched — they carry extra create-only fields (`roll_id`, `gps_lat`, `gps_lon`).

- [ ] **Step 5: Disable the arrows while saving or on a date error**

In the dialog markup, change the two nav buttons' `disabled` attributes:

```svelte
disabled={!hasPrevShot || shotNavSaving || !!shotDateError}
```

and

```svelte
disabled={!hasNextShot || shotNavSaving || !!shotDateError}
```

- [ ] **Step 6: Verify types, unit tests, and build**

Run: `cd frontend && bun run check && bun run test:unit && bun run build && git checkout -- frontend/build/.gitkeep`
Expected: svelte-check 0 errors, all unit tests PASS, build succeeds.

- [ ] **Step 7: Manual spot-check (dev server)**

Run `just dev`, open a roll with 2+ shots at `http://localhost:5273`, open Edit Shot, change Notes, click `>`: the next shot loads; navigate back with `<`: the edit is there. Reload the page: the edit persisted. Clear the frame number, click `>`: navigation is blocked with "Frame number is required."

- [ ] **Step 8: Commit**

```bash
just fmt
git add frontend/src/routes/\(app\)/rolls/\[id\]/+page.svelte
git commit -m "fix(shots): auto-save edits when navigating shots in the edit dialog (kammerz-11o3)"
```

---

### Task 3: e2e regression test

**Files:**

- Modify: `frontend/tests/smoke.spec.ts` (append after the kammerz-3hq test)

**Interfaces:**

- Consumes: `BASE` from `./shared`; the running release binary + storageState auth the smoke suite already provides. FrameStrip filled-frame buttons have `aria-label` matching `/^Frame 1,.*click to edit/`; dialog nav buttons are `aria-label="Next shot"` / `"Previous shot"`. The `Dialog` component has `role="dialog"` — ALL in-dialog locators must scope through it, because the QuickAddBar behind the dialog also renders a `<textarea>` (its Notes field) and a "Save & Next" button.
- Produces: nothing downstream.

- [ ] **Step 1: Write the test**

```typescript
/**
 * Regression guard for kammerz-11o3: edits made in the Edit Shot dialog must
 * survive < > navigation to an adjacent shot (auto-save-on-navigate). The old
 * behavior re-seeded the shared form fields from the target shot, silently
 * discarding unsaved edits.
 */
test('edit-shot dialog auto-saves edits when navigating between shots (kammerz-11o3)', async ({ page }) => {
	const created = await page.request.post(`${BASE}/api/rolls`, {
		data: { roll_id: `E2E-NAV-${Date.now()}`, status: 'loaded', frame_count: 36 }
	});
	expect(created.ok(), `create roll failed: ${created.status()}`).toBeTruthy();
	const rollId: number = await created.json();

	const shotIds: number[] = [];
	for (const frame of ['1', '2']) {
		const res = await page.request.post(`${BASE}/api/shots`, {
			data: { roll_id: rollId, frame_number: frame, lens_ids: [] }
		});
		expect(res.ok(), `create shot ${frame} failed: ${res.status()}`).toBeTruthy();
		shotIds.push(await res.json());
	}

	await page.goto(`${BASE}/rolls/${rollId}`);
	await page.waitForLoadState('networkidle');

	// Open the Edit Shot dialog on frame 1 via the FrameStrip. Scope every
	// in-dialog locator through role=dialog — the QuickAddBar behind it also
	// has a <textarea> and a "Save & Next" button.
	await page.getByRole('button', { name: /^Frame 1,.*click to edit/ }).click();
	const dialog = page.getByRole('dialog');
	await expect(dialog.getByText('Shot 1 of 2')).toBeVisible();

	// Edit shot 1's notes, then navigate — this must auto-save shot 1.
	await dialog.locator('textarea').fill('note one');
	await dialog.getByRole('button', { name: 'Next shot' }).click();
	await expect(dialog.getByText('Shot 2 of 2')).toBeVisible();

	// Edit shot 2's notes and save normally.
	await dialog.locator('textarea').fill('note two');
	await dialog.getByRole('button', { name: 'Save', exact: true }).click();

	// Both edits persisted server-side.
	const shot1 = await (await page.request.get(`${BASE}/api/shots/${shotIds[0]}`)).json();
	const shot2 = await (await page.request.get(`${BASE}/api/shots/${shotIds[1]}`)).json();
	expect(shot1.notes, 'shot 1 edit must survive < > navigation').toBe('note one');
	expect(shot2.notes).toBe('note two');

	await page.request.delete(`${BASE}/api/rolls/${rollId}`);
});
```

- [ ] **Step 2: Run the e2e suite**

Run: `just e2e`
Expected: all smoke tests PASS including the new one. (This builds the release binary and serves on :3002 — make sure `just dev` isn't holding the port.)

If the new test fails on a selector, debug with `cd frontend && bunx playwright test -g "kammerz-11o3" --headed` against the already-built binary.

- [ ] **Step 3: Commit**

```bash
just fmt
git add frontend/tests/smoke.spec.ts
git commit -m "test(e2e): edit-shot navigation persists edits (kammerz-11o3)"
```

---

### Task 4: Gates and PR

**Files:** none new — verification and delivery only.

- [ ] **Step 1: Full local CI mirror**

Run: `just ci`
Expected: fmt-check, backend (build/clippy/test), frontend (install/check/unit/build), e2e all green. Restore `frontend/build/.gitkeep` if the build wiped it.

- [ ] **Step 2: Push and open the PR**

```bash
git restore --staged .beads/issues.jsonl 2>/dev/null; git checkout -- .beads/issues.jsonl 2>/dev/null
git push -u origin kammerz-11o3-edit-shot-autosave-nav
gh pr create --title "fix(shots): auto-save edits when navigating shots in the edit dialog (kammerz-11o3)" --body "## Summary
- Edit Shot dialog < > navigation (buttons and arrow keys) now auto-saves the current shot's edits before loading the adjacent shot; previously they were silently discarded
- Extracts the form payload/dirty-compare into \`\$lib/utils/shot-form\` with vitest coverage; navigation is blocked (with the usual inline error) on an empty frame number, an invalid date, or a failed save
- Adds a Playwright regression test proving both shots' edits persist server-side

Bead: kammerz-11o3 (close post-merge on main per convention)

🤖 Generated with [Claude Code](https://claude.com/claude-code)"
```

Expected: PR opens; the `all-checks` aggregate goes green.

- [ ] **Step 3: Post-merge (on `main`, separate from this branch)**

After squash-merge: `git checkout main && git pull`, run `just ci` (re-gate main), then `bd close kammerz-11o3`, commit the beads export as `chore(beads): close kammerz-11o3 after PR #<N> merge`, `bd dolt push`, `git push`.
