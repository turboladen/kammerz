# Guided f/ and Shutter Inputs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the free-text aperture/shutter inputs with a guided combobox that suggests standard values and cleans obvious typos, while still accepting any special-case value.

**Architecture:** A new pure util (`exposure.ts`) holds the suggestion lists, broad "recognized" sets, and bare-value normalizers. `ComboInput` gains two generic optional props (`normalize`, `warning`) plus a `mono` styling flag. The two consumers — `QuickAddBar` and the Shot add/edit dialog — swap their plain inputs for `ComboInput` wired to the util. Backend is untouched.

**Tech Stack:** Svelte 5 runes, Tailwind 4, vitest (util), TypeScript.

## Global Constraints

- **Bare storage:** aperture persists as `5.6` (never `f/5.6`); shutter as `1/250` or `4` (never `1/250s`). Every display site prepends `f/` / appends `s`; an `f/`-prefixed value double-prefixes to `f/f/2.8` (kammerz-jd1).
- **Backend unchanged:** no format validation added — special cases must persist. `src/routes/shots.rs` keeps `Option<String>` + `trim_opt`.
- **Preserve `ComboInput`'s combobox ARIA + `onmousedown` option pattern** (kammerz-dsy) — only add props, don't alter existing behavior.
- **Colors via existing classes:** the amber warning reuses `text-amber-400` / `border-amber-500` Tailwind classes already used for the over-roll warning in `rolls/[id]/+page.svelte`.
- **Gates:** `cd frontend && bun run check` (svelte-check) and `bun run build` must pass; vitest must pass (it gates in `ci-frontend`). Run `just check` before opening the PR. After any `bun run build`, restore the wiped keepfile: `git checkout -- frontend/build/.gitkeep`.
- All work happens in the worktree `.claude/worktrees/exposure` on branch `feat/guided-exposure-inputs`.

---

### Task 1: `exposure.ts` util + tests

**Files:**
- Create: `frontend/src/lib/utils/exposure.ts`
- Test: `frontend/src/lib/utils/exposure.test.ts`

**Interfaces:**
- Produces: `APERTURE_SUGGESTIONS: string[]`, `SHUTTER_SUGGESTIONS: string[]`, `normalizeAperture(v: string): string`, `normalizeShutter(v: string): string`, `isRecognizedAperture(v: string): boolean`, `isRecognizedShutter(v: string): boolean`.

- [ ] **Step 1: Write the failing test**

Create `frontend/src/lib/utils/exposure.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import {
	APERTURE_SUGGESTIONS,
	SHUTTER_SUGGESTIONS,
	isRecognizedAperture,
	isRecognizedShutter,
	normalizeAperture,
	normalizeShutter
} from './exposure';

describe('normalizeAperture', () => {
	it('strips a leading f/ or f', () => {
		expect(normalizeAperture('f/5.6')).toBe('5.6');
		expect(normalizeAperture('F5.6')).toBe('5.6');
	});
	it('converts comma to dot and removes whitespace', () => {
		expect(normalizeAperture('5,6')).toBe('5.6');
		expect(normalizeAperture('  2.8 ')).toBe('2.8');
	});
	it('leaves a bare value unchanged and is idempotent', () => {
		expect(normalizeAperture('5.6')).toBe('5.6');
		expect(normalizeAperture(normalizeAperture('f/5.6'))).toBe('5.6');
	});
	it('returns empty for empty input', () => {
		expect(normalizeAperture('')).toBe('');
	});
});

describe('normalizeShutter', () => {
	it('strips a trailing s', () => {
		expect(normalizeShutter('1/250s')).toBe('1/250');
		expect(normalizeShutter('30 s')).toBe('30');
	});
	it('leaves fractions, seconds, and B untouched', () => {
		expect(normalizeShutter('1/250')).toBe('1/250');
		expect(normalizeShutter('4')).toBe('4');
		expect(normalizeShutter('B')).toBe('B');
	});
	it('returns empty for empty input', () => {
		expect(normalizeShutter('')).toBe('');
	});
});

describe('isRecognizedAperture', () => {
	it('recognizes full/half/third-stop values (normalizing first)', () => {
		expect(isRecognizedAperture('5.6')).toBe(true);
		expect(isRecognizedAperture('1.8')).toBe(true);
		expect(isRecognizedAperture('3.5')).toBe(true);
		expect(isRecognizedAperture('64')).toBe(true);
		expect(isRecognizedAperture('f/5.6')).toBe(true);
	});
	it('flags genuine typos as non-standard', () => {
		expect(isRecognizedAperture('56')).toBe(false);
		expect(isRecognizedAperture('8.5')).toBe(false);
	});
	it('is false for empty', () => {
		expect(isRecognizedAperture('')).toBe(false);
	});
});

describe('isRecognizedShutter', () => {
	it('recognizes standard and legacy speeds (normalizing first)', () => {
		expect(isRecognizedShutter('1/250')).toBe(true);
		expect(isRecognizedShutter('1/50')).toBe(true);
		expect(isRecognizedShutter('B')).toBe(true);
		expect(isRecognizedShutter('1/250s')).toBe(true);
	});
	it('flags genuine typos as non-standard', () => {
		expect(isRecognizedShutter('250')).toBe(false);
		expect(isRecognizedShutter('1/275')).toBe(false);
	});
	it('is false for empty', () => {
		expect(isRecognizedShutter('')).toBe(false);
	});
});

describe('suggestion lists are bare', () => {
	it('have no f/ prefix and no trailing s', () => {
		expect(APERTURE_SUGGESTIONS).toContain('5.6');
		expect(APERTURE_SUGGESTIONS.every((v) => !v.toLowerCase().startsWith('f'))).toBe(true);
		expect(SHUTTER_SUGGESTIONS).toContain('1/250');
		expect(SHUTTER_SUGGESTIONS.every((v) => !v.endsWith('s'))).toBe(true);
	});
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd frontend && bun run test:unit -- exposure`
Expected: FAIL — cannot resolve `./exposure`.

- [ ] **Step 3: Write the implementation**

Create `frontend/src/lib/utils/exposure.ts`:

```ts
// Standard aperture / shutter values for the guided shot-entry inputs.
// Suggestion lists (dropdown) are intentionally concise; the "recognized" sets
// behind the off-list ⚠ hint are broad so only genuine typos are flagged.
// All values are BARE — aperture "5.6" (never "f/5.6"), shutter "1/250"
// (never "1/250s") — because every display site prepends f/ / appends s.

/** Half-stop apertures — the f/ dropdown suggestions. */
export const APERTURE_SUGGESTIONS = [
	'1', '1.2', '1.4', '1.7', '2', '2.4', '2.8', '3.4', '4', '4.8',
	'5.6', '6.7', '8', '9.5', '11', '13', '16', '19', '22', '27', '32'
];

/** Standard shutter speeds, fast → slow, then whole seconds, then bulb. */
export const SHUTTER_SUGGESTIONS = [
	'1/4000', '1/2000', '1/1000', '1/500', '1/250', '1/125', '1/60', '1/30',
	'1/15', '1/8', '1/4', '1/2', '1', '2', '4', '8', '15', '30', 'B'
];

/** Broad recognized aperture set (full ∪ half ∪ third stops, f/0.95–f/64). */
const RECOGNIZED_APERTURES = new Set([
	'0.95', '1', '1.1', '1.2', '1.4', '1.6', '1.7', '1.8', '2', '2.2', '2.4', '2.5',
	'2.8', '3.2', '3.4', '3.5', '4', '4.5', '4.8', '5', '5.6', '6.3', '6.7', '7.1',
	'8', '9', '9.5', '10', '11', '13', '14', '16', '18', '19', '20', '22', '25', '27',
	'29', '32', '36', '40', '45', '51', '57', '64'
]);

/** Broad recognized shutter set (standard + common legacy/leaf speeds + long). */
const RECOGNIZED_SHUTTERS = new Set([
	'1/8000', '1/4000', '1/2000', '1/1000', '1/500', '1/400', '1/320', '1/300',
	'1/250', '1/200', '1/160', '1/125', '1/100', '1/90', '1/80', '1/60', '1/50',
	'1/45', '1/40', '1/30', '1/25', '1/20', '1/15', '1/10', '1/8', '1/5', '1/4',
	'1/2', '1', '2', '4', '8', '15', '30', '60', 'B', 'T'
]);

/** Strip a leading f//f, comma→dot, remove whitespace. Emits a bare aperture. */
export function normalizeAperture(v: string): string {
	return v.replace(/\s+/g, '').replace(/^f\/?/i, '').replace(',', '.');
}

/** Strip a trailing s/sec/seconds and whitespace. Emits a bare shutter value. */
export function normalizeShutter(v: string): string {
	return v.replace(/\s+/g, '').replace(/(s|sec|secs|second|seconds)$/i, '');
}

/** True if the (normalized) value is a recognized standard aperture. */
export function isRecognizedAperture(v: string): boolean {
	const n = normalizeAperture(v);
	return n !== '' && RECOGNIZED_APERTURES.has(n);
}

/** True if the (normalized) value is a recognized standard shutter speed. */
export function isRecognizedShutter(v: string): boolean {
	const n = normalizeShutter(v);
	return n !== '' && RECOGNIZED_SHUTTERS.has(n);
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd frontend && bun run test:unit -- exposure`
Expected: PASS (all describe blocks green).

- [ ] **Step 5: Commit**

```bash
git add frontend/src/lib/utils/exposure.ts frontend/src/lib/utils/exposure.test.ts
git commit -m "feat(exposure): standard f/ & shutter lists + bare normalizers (kammerz-dzuy)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: Extend `ComboInput` with `normalize`, `warning`, `mono`

**Files:**
- Modify: `frontend/src/lib/components/ui/ComboInput.svelte`

**Interfaces:**
- Consumes: nothing.
- Produces: `ComboInput` accepts three new optional props — `normalize?: (v: string) => string` (applied on blur), `warning?: string` (amber hint shown only when unfocused), `mono?: boolean` (renders the input in the mono data font).

Components are not unit-tested in this project (e2e/manual per convention); this task is verified by svelte-check + build.

- [ ] **Step 1: Add the props**

In `frontend/src/lib/components/ui/ComboInput.svelte`, replace the `Props` interface and destructure (lines 2–10):

```svelte
	interface Props {
		label?: string;
		hint?: string;
		placeholder?: string;
		value: string;
		options: string[];
		/** Applied to the committed value on blur — e.g. strip an f/ prefix. */
		normalize?: (v: string) => string;
		/** Amber advisory shown below the input when set and the field is unfocused. */
		warning?: string;
		/** Render the input text in the mono data font. */
		mono?: boolean;
	}

	let {
		label,
		hint,
		placeholder,
		value = $bindable(),
		options,
		normalize,
		warning,
		mono = false
	}: Props = $props();
```

- [ ] **Step 2: Normalize on blur**

Replace `handleBlur` (lines 43–49):

```svelte
	function handleBlur() {
		// Delay to allow click on dropdown option to register
		setTimeout(() => {
			showDropdown = false;
			highlightIndex = -1;
			if (normalize) value = normalize(value);
		}, 150);
	}
```

- [ ] **Step 3: Apply mono + amber-border classes to the input**

Replace the input's `class` attribute (lines 112–113) with a conditional version. The border color and mono font become conditional; everything else is unchanged:

```svelte
			class="rounded-lg border bg-surface px-3 py-2 text-sm text-text placeholder-text-faint
				transition-colors focus:outline-none focus:ring-1 focus:ring-accent/50
				{mono ? 'font-mono' : ''}
				{warning && !showDropdown ? 'border-amber-500/60 focus:border-amber-500' : 'border-border focus:border-accent'}"
```

- [ ] **Step 4: Render the warning line**

Replace the trailing hint block (lines 136–138) with the hint plus the warning:

```svelte
	{#if hint}
		<span class="text-xs text-text-faint">{hint}</span>
	{/if}
	{#if warning && !showDropdown}
		<span class="text-xs text-amber-400">{warning}</span>
	{/if}
```

- [ ] **Step 5: Verify svelte-check + build**

Run: `cd frontend && bun run check && bun run build`
Expected: svelte-check `0 ERRORS`, build `✔ done`. Then: `git checkout -- frontend/build/.gitkeep`

- [ ] **Step 6: Commit**

```bash
git add frontend/src/lib/components/ui/ComboInput.svelte
git commit -m "feat(combo): optional normalize, warning, and mono props (kammerz-dzuy)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: Wire `QuickAddBar` to the guided inputs

**Files:**
- Modify: `frontend/src/lib/components/rolls/QuickAddBar.svelte`

**Interfaces:**
- Consumes: `ComboInput` (Task 2) and `exposure.ts` exports (Task 1).

- [ ] **Step 1: Add imports**

In the `<script>` block (after the existing `$lib/components/ui/*` imports, ~line 6), add:

```svelte
	import ComboInput from '$lib/components/ui/ComboInput.svelte';
	import {
		APERTURE_SUGGESTIONS,
		SHUTTER_SUGGESTIONS,
		isRecognizedAperture,
		isRecognizedShutter,
		normalizeAperture,
		normalizeShutter
	} from '$lib/utils/exposure';
```

- [ ] **Step 2: Normalize on save**

In `handleSave` (lines 60–69), replace the `onsave({ ... })` payload's `aperture` and `shutterSpeed` lines so the keyboard-save path also stores bare values:

```svelte
		onsave({
			frameNumber,
			aperture: normalizeAperture(aperture),
			shutterSpeed: normalizeShutter(shutterSpeed),
			lensId: isFixedLens ? lensIdProp : localLensId,
			date,
			time,
			location,
			notes
		});
```

- [ ] **Step 3: Swap the aperture input**

Replace the f/ Aperture block (lines 100–111) with:

```svelte
			<!-- f/ Aperture — guided combobox (standard values + free entry) -->
			<div class="w-20">
				<ComboInput
					label="f/"
					placeholder="5.6"
					mono
					options={APERTURE_SUGGESTIONS}
					normalize={normalizeAperture}
					warning={aperture && !isRecognizedAperture(aperture) ? 'Non-standard' : ''}
					bind:value={aperture}
				/>
			</div>
```

- [ ] **Step 4: Swap the shutter input**

Replace the Shutter speed block (lines 113–123) with:

```svelte
			<!-- Shutter — guided combobox -->
			<div class="w-24">
				<ComboInput
					label="Shutter"
					placeholder="1/250"
					mono
					options={SHUTTER_SUGGESTIONS}
					normalize={normalizeShutter}
					warning={shutterSpeed && !isRecognizedShutter(shutterSpeed) ? 'Non-standard' : ''}
					bind:value={shutterSpeed}
				/>
			</div>
```

- [ ] **Step 5: Verify svelte-check + build**

Run: `cd frontend && bun run check && bun run build`
Expected: `0 ERRORS`, build `✔ done`. Then: `git checkout -- frontend/build/.gitkeep`

- [ ] **Step 6: Commit**

```bash
git add frontend/src/lib/components/rolls/QuickAddBar.svelte
git commit -m "feat(quick-entry): guided f/ & shutter comboboxes (kammerz-dzuy)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: Wire the Shot add/edit dialog

**Files:**
- Modify: `frontend/src/routes/(app)/rolls/[id]/+page.svelte`

**Interfaces:**
- Consumes: `ComboInput` (Task 2) and `exposure.ts` exports (Task 1).

- [ ] **Step 1: Add imports**

In the page's `<script>` import block, alongside the existing `$lib/components/ui/*` imports (the page already imports `Input` from there), add:

```svelte
	import ComboInput from '$lib/components/ui/ComboInput.svelte';
	import {
		APERTURE_SUGGESTIONS,
		SHUTTER_SUGGESTIONS,
		isRecognizedAperture,
		isRecognizedShutter,
		normalizeAperture,
		normalizeShutter
	} from '$lib/utils/exposure';
```

- [ ] **Step 2: Normalize at all three submit sites**

The page builds the shot payload in three places (lines ~551–552, ~563–564, ~594–595), each with the identical pair:

```svelte
				aperture: shotAperture || null,
				shutter_speed: shotShutterSpeed || null,
```

Replace **every** occurrence of that pair with:

```svelte
				aperture: normalizeAperture(shotAperture) || null,
				shutter_speed: normalizeShutter(shotShutterSpeed) || null,
```

(There are three; use find-and-replace across the file and confirm three edits.)

- [ ] **Step 3: Swap the two dialog inputs**

Replace lines 1259–1260:

```svelte
				<Input label="Aperture (f/)" bind:value={shotAperture} placeholder="5.6" />
				<Input label="Shutter Speed" bind:value={shotShutterSpeed} placeholder="1/125" />
```

with:

```svelte
				<ComboInput
					label="Aperture (f/)"
					placeholder="5.6"
					mono
					options={APERTURE_SUGGESTIONS}
					normalize={normalizeAperture}
					warning={shotAperture && !isRecognizedAperture(shotAperture) ? 'Non-standard f/ value' : ''}
					bind:value={shotAperture}
				/>
				<ComboInput
					label="Shutter Speed"
					placeholder="1/250"
					mono
					options={SHUTTER_SUGGESTIONS}
					normalize={normalizeShutter}
					warning={shotShutterSpeed && !isRecognizedShutter(shotShutterSpeed) ? 'Non-standard shutter speed' : ''}
					bind:value={shotShutterSpeed}
				/>
```

- [ ] **Step 4: Verify svelte-check + build**

Run: `cd frontend && bun run check && bun run build`
Expected: `0 ERRORS`, build `✔ done`. Then: `git checkout -- frontend/build/.gitkeep`

- [ ] **Step 5: Commit**

```bash
git add "frontend/src/routes/(app)/rolls/[id]/+page.svelte"
git commit -m "feat(shot-dialog): guided f/ & shutter comboboxes (kammerz-dzuy)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: Full gate + manual verification

**Files:** none (verification only).

- [ ] **Step 1: Format**

Run: `just fmt` then `git checkout -- frontend/build/.gitkeep` (in case build ran).

- [ ] **Step 2: Full local gate**

Run: `just check`
Expected: `fmt-check`, `ci-backend`, `ci-frontend` all pass (backend is unchanged; frontend runs svelte-check + vitest + build). Restore `frontend/build/.gitkeep` if the build wiped it.

- [ ] **Step 3: Manual / Playwright check** (`just dev` → a roll detail page)

Verify against a throwaway DB copy (never the live catalog):
- **Dropdown:** focus the f/ field → half-stop suggestions appear; typing `5` narrows to `5.6`; picking one fills it.
- **Normalize:** type `f/5.6` and blur → becomes `5.6`; type `1/250s` in shutter and blur → `1/250`.
- **Off-list ⚠:** type `1.8` (recognized third-stop) → **no** warning; type `56` → amber "Non-standard" appears after blur, amber border shows, and it still saves.
- **Both surfaces:** confirm in the `QuickAddBar` card *and* the full Shot add/edit dialog.
- **Round-trip:** save a shot, reopen it → the value shows bare (no `f/`, no `s`), FrameStrip shows `f/5.6`.

- [ ] **Step 4: Commit any formatting**

```bash
git add -A
git commit -m "chore: formatting for guided exposure inputs (kammerz-dzuy)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```
(Skip if the tree is already clean.)

---

## Self-Review

**Spec coverage:** ✅ util (Task 1) ✅ ComboInput normalize+warning (Task 2) ✅ QuickAddBar swap + submit-normalize (Task 3) ✅ Shot dialog swap + submit-normalize (Task 4) ✅ backend unchanged (no task, per spec) ✅ vitest (Task 1) ✅ manual/e2e (Task 5). Decoupled suggestion vs recognized sets: `APERTURE_SUGGESTIONS` (half-stops) vs `RECOGNIZED_APERTURES` (broad) — ✅.

**Type consistency:** `normalizeAperture`/`normalizeShutter`/`isRecognizedAperture`/`isRecognizedShutter` names are used identically in Tasks 1, 3, 4. `ComboInput` prop names `normalize`/`warning`/`mono` match between Task 2 (definition) and Tasks 3–4 (usage).

**Known cosmetic note (surface to the user during review):** in the compact `QuickAddBar`, the off-list warning renders under a narrow (`w-20`/`w-24`) column, so when it appears the text may wrap and Row 1's `items-end` can leave a small gap — the amber border is the primary signal there. Only appears for genuinely off-list values after blur. If undesired, the fix is to drop `warning` on the QuickAddBar combos (keep it dialog-only).
