# Accessible Theme (Contrast + Red/Green-Safe Palette) Implementation Plan

> **Superseded 2026-06-03 — read as history.** This plan was written for the
> original approach that *kept the warm-brown surfaces*. Real-app review showed
> that warmth was the problem, so the implemented theme pivoted to a **neutral
> graphite/silver** base (surface `#0d0d0c`, text `#eeebe7`/`#bab4ad`/`#8f8981`,
> amber accent `#e2a45e`) — see the spec's revision banner and `UI_DESIGN.md`.
> Consequently the specific token values in the tasks below are stale (e.g. the
> warm surfaces, `--color-danger-hover` is now `#b03f33` not `#d2594c`), and the
> Task 7 contrast script's `S="151210"` + old text tiers are superseded by the
> graphite surface `#0d0d0c` and current tiers. The red/green-safe **status**
> palette and the danger/icon **structure** carried over unchanged; only the
> surface/text/accent values and the texture/material layer changed. For current
> authoritative values, read `frontend/src/app.css` and `UI_DESIGN.md`.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Kammerz "Darkroom Ledger" theme readable at normal brightness (all text tiers clear WCAG AA) and distinguishable for red/green color vision, while keeping the dark warm identity.

**Architecture:** Every component sources color from CSS custom properties in the `@theme` block of `frontend/src/app.css`. The palette change is value swaps in that one block; it propagates everywhere automatically (`statusConfig` in `status.ts` references `var(--color-status-*)`, badges/dots/chevrons use Tailwind `*-status-*` utilities). Two shared components (`Button`, `ConfirmDialog`) get a stronger danger treatment that signals "destructive" via solid fill + line icons, not red hue alone.

**Tech Stack:** SvelteKit + Svelte 5 runes, Tailwind CSS 4 (`@theme` tokens → utilities), `lucide-svelte` icons, Bun.

**Reference spec:** `docs/superpowers/specs/2026-06-02-accessible-theme-contrast-colorblind-design.md`

**How "tests" work here:** This is presentational. The repeatable checks are: `bun run check` (svelte-check — catches markup/import errors), `bun run build` (production build), a deterministic WCAG-contrast assertion script (Task 7), and a visual pass via `just dev`. Keep `just dev` running (axum :3002 + Vite :5173) and open `http://localhost:5173` to eyeball after color tasks.

---

## File structure

- `frontend/src/app.css` — `@theme` block: 3 text-tier values, 9 status values, 3 new danger tokens. (Tasks 1–3)
- `frontend/src/lib/components/ui/Button.svelte` — `danger` variant restyle. (Task 3)
- `frontend/src/lib/components/ui/ConfirmDialog.svelte` — warning icon + iconed confirm. (Task 4)
- `frontend/src/routes/(app)/rolls/[id]/+page.svelte` and `frontend/src/routes/(app)/cameras/[id]/+page.svelte` — add `Trash2` to the two explicit Delete buttons. (Task 5)
- `UI_DESIGN.md` — document the new palette, colorblind rationale, danger pattern. (Task 6)

---

### Task 1: Brighten text tiers to AA

**Files:**
- Modify: `frontend/src/app.css` (the three `--color-text*` lines in `@theme`)

- [ ] **Step 1: Edit the three text-tier tokens**

Find (in the `@theme` block, under `/* Text: cream/warm white, like aged paper */`):

```css
	--color-text: #e8e2d9;
	--color-text-muted: #a09488;
	--color-text-faint: #6d6258;
```

Replace with:

```css
	--color-text: #ece6dd;
	--color-text-muted: #bcb0a0;
	--color-text-faint: #9d9384;
```

- [ ] **Step 2: Verify it compiles**

Run: `cd frontend && bun run check`
Expected: `0 ERRORS 0 WARNINGS`

- [ ] **Step 3: Visual check**

With `just dev` running, open `http://localhost:5173`, go to any roll detail page. The UPPERCASE section labels ("STATUS", "DEVELOPMENT"), the dates, and metadata (previously `text-faint`, near-invisible) should now read clearly at normal brightness.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/app.css
git commit -m "feat(theme): brighten text tiers to clear WCAG AA"
```

---

### Task 2: Red/green-safe status palette

**Files:**
- Modify: `frontend/src/app.css` (the nine `--color-status-*` lines in `@theme`)

- [ ] **Step 1: Edit the nine status tokens**

Find (under `/* Status colors: shifted warmer, still distinct */`):

```css
	--color-status-loaded: #6ca4d4;
	--color-status-shooting: #5cb88a;
	--color-status-shot: #a38bc7;
	--color-status-at-lab: #d4a84e;
	--color-status-lab-done: #d4c24e;
	--color-status-developing: #c87a42;
	--color-status-developed: #b384c7;
	--color-status-scanned: #5aaf9e;
	--color-status-archived: #7a726a;
```

Replace with:

```css
	/* Status colors: red/green-safe — blue (shooting phase) → amber (development) → neutral (finished) */
	--color-status-loaded: #6fbcff;
	--color-status-shooting: #93c4ec;
	--color-status-shot: #afb3ea;
	--color-status-at-lab: #e3a347;
	--color-status-lab-done: #f0cf57;
	--color-status-developing: #dd8b44;
	--color-status-developed: #ecc185;
	--color-status-scanned: #a8cdd8;
	--color-status-archived: #b3a99c;
```

- [ ] **Step 2: Verify it compiles**

Run: `cd frontend && bun run check`
Expected: `0 ERRORS 0 WARNINGS`

- [ ] **Step 3: Visual check**

On `http://localhost:5173`: the roll detail status chevron bar and badges, the rolls list (group-by-status), and the Stats page status-distribution bar + legend. Confirm the 9 statuses read as blue→amber→neutral and that the previously-confusable pairs (shooting/scanned, shot/developed) are now clearly different.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/app.css
git commit -m "feat(theme): red/green-safe status palette (blue->amber->neutral)"
```

---

### Task 3: Danger tokens + solid danger button

**Files:**
- Modify: `frontend/src/app.css` (add danger tokens to `@theme`)
- Modify: `frontend/src/lib/components/ui/Button.svelte:18`

- [ ] **Step 1: Add danger tokens to `@theme`**

In `frontend/src/app.css`, find the accent block:

```css
	/* Accent: amber/gold, like a darkroom safelight */
	--color-accent: #d4915c;
	--color-accent-hover: #e4a876;
	--color-accent-muted: #a06830;
```

Immediately after it (still inside `@theme`), add:

```css

	/* Danger: destructive actions. Solid fill takes white text at ~5:1; the
	   lighter -fg is for danger icons/text on the dark surface. */
	--color-danger: #c0473a;
	--color-danger-hover: #d2594c;
	--color-danger-fg: #e88c80;
```

- [ ] **Step 2: Restyle the `danger` button variant**

In `frontend/src/lib/components/ui/Button.svelte`, find line 18:

```js
		danger: 'bg-red-500/15 text-red-400 hover:bg-red-500/25'
```

Replace with:

```js
		danger: 'bg-danger text-white hover:bg-danger-hover'
```

- [ ] **Step 3: Verify it compiles**

Run: `cd frontend && bun run check`
Expected: `0 ERRORS 0 WARNINGS`

- [ ] **Step 4: Visual check**

On `http://localhost:5173`, open a roll detail page. The "Delete" button should now be a solid red fill with white text (high emphasis), not a faint tint. (The icon comes in Task 5.)

- [ ] **Step 5: Commit**

```bash
git add frontend/src/app.css frontend/src/lib/components/ui/Button.svelte
git commit -m "feat(theme): solid high-emphasis danger button + danger tokens"
```

---

### Task 4: ConfirmDialog — warning icon + iconed confirm

**Files:**
- Modify: `frontend/src/lib/components/ui/ConfirmDialog.svelte`

- [ ] **Step 1: Import the icons**

In `ConfirmDialog.svelte`, find line 2:

```js
	import Button from './Button.svelte';
```

Replace with:

```js
	import Button from './Button.svelte';
	import { AlertTriangle, Trash2 } from 'lucide-svelte';
```

- [ ] **Step 2: Add the warning icon beside the title**

Find (lines 64):

```svelte
			<h2 class="font-display text-xl">{title}</h2>
```

Replace with:

```svelte
			<div class="flex items-center gap-3">
				{#if variant === 'danger'}
					<span class="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-danger/15 text-danger-fg">
						<AlertTriangle size={18} strokeWidth={2} />
					</span>
				{/if}
				<h2 class="font-display text-xl">{title}</h2>
			</div>
```

- [ ] **Step 3: Put the trash icon in the confirm button**

Find (line 68):

```svelte
				<Button variant={variant} onclick={handleConfirm}>{confirmLabel}</Button>
```

Replace with:

```svelte
				<Button variant={variant} onclick={handleConfirm}>
					{#if variant === 'danger'}<Trash2 size={16} strokeWidth={2} />{/if}
					{confirmLabel}
				</Button>
```

- [ ] **Step 4: Verify it compiles**

Run: `cd frontend && bun run check`
Expected: `0 ERRORS 0 WARNINGS`

- [ ] **Step 5: Visual check**

On `http://localhost:5173`, trigger a destructive confirm (e.g. roll detail → delete a shot, or the roll's Delete → confirm dialog). The dialog header shows a warning triangle in danger tint; the confirm button shows a white trash icon + label. The crisp white stroke must read clearly on the red.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/lib/components/ui/ConfirmDialog.svelte
git commit -m "feat(theme): warning + trash icons on destructive confirm dialog"
```

---

### Task 5: Trash icon on the two explicit Delete buttons

**Files:**
- Modify: `frontend/src/routes/(app)/rolls/[id]/+page.svelte:586`
- Modify: `frontend/src/routes/(app)/cameras/[id]/+page.svelte:375`

Neither file currently imports from `lucide-svelte`.

- [ ] **Step 1: rolls/[id] — add the import**

In `frontend/src/routes/(app)/rolls/[id]/+page.svelte`, find the import for the types line:

```js
	import type { RollWithDetails, RollInsert, Camera, FilmStock, Lens, Shot, Lab, DevelopmentLab, DevelopmentSelf, DevStage, RollStatus, PushPull, LensMount } from '$lib/types';
```

Add immediately after it:

```js
	import { Trash2 } from 'lucide-svelte';
```

- [ ] **Step 2: rolls/[id] — add the icon to the Delete button**

Find (line ~586):

```svelte
		<Button variant="danger" onclick={handleDelete}>Delete</Button>
```

Replace with:

```svelte
		<Button variant="danger" onclick={handleDelete}><Trash2 size={16} strokeWidth={2} />Delete</Button>
```

- [ ] **Step 3: cameras/[id] — add the import**

In `frontend/src/routes/(app)/cameras/[id]/+page.svelte`, add an import line alongside the existing imports at the top of the `<script>` block:

```js
	import { Trash2 } from 'lucide-svelte';
```

- [ ] **Step 4: cameras/[id] — add the icon to the Delete button**

Find (line ~375):

```svelte
		<Button variant="danger" onclick={handleDelete}>Delete</Button>
```

Replace with:

```svelte
		<Button variant="danger" onclick={handleDelete}><Trash2 size={16} strokeWidth={2} />Delete</Button>
```

- [ ] **Step 5: Verify it compiles**

Run: `cd frontend && bun run check`
Expected: `0 ERRORS 0 WARNINGS`

- [ ] **Step 6: Visual check**

On `http://localhost:5173`, the roll detail and camera detail "Delete" buttons now show a white trash icon + "Delete" on the solid red fill.

- [ ] **Step 7: Commit**

```bash
git add "frontend/src/routes/(app)/rolls/[id]/+page.svelte" "frontend/src/routes/(app)/cameras/[id]/+page.svelte"
git commit -m "feat(theme): trash icon on explicit Delete buttons"
```

---

### Task 6: Update UI_DESIGN.md

**Files:**
- Modify: `UI_DESIGN.md` (Text, Status Colors, Danger sections; Button variant row)

- [ ] **Step 1: Update the Text tier table**

Find (lines ~35–37):

```markdown
| `--color-text` | `#e8e2d9` | Primary text, headings |
| `--color-text-muted` | `#a09488` | Secondary text, labels, descriptions |
| `--color-text-faint` | `#6d6258` | Tertiary text, hints, timestamps |
```

Replace with:

```markdown
| `--color-text` | `#ece6dd` | Primary text, headings |
| `--color-text-muted` | `#bcb0a0` | Secondary text, labels, descriptions (≥8:1 on surface) |
| `--color-text-faint` | `#9d9384` | Tertiary text, hints, timestamps (≥6:1 on surface — all tiers clear WCAG AA) |
```

- [ ] **Step 2: Replace the Status Colors section**

Find (lines ~47–58, the `### Status Colors` heading through the Archived row):

```markdown
### Status Colors (warm-shifted, still distinct)

| Status | Hex | Visual |
|---|---|---|
| Loaded | `#6ca4d4` | Dusty blue |
| Shooting | `#5cb88a` | Sage green |
| Shot | `#a38bc7` | Muted lavender |
| At Lab | `#d4a84e` | Warm gold |
| Developing | `#c87a42` | Burnt orange |
| Developed | `#b384c7` | Soft purple |
| Scanned | `#5aaf9e` | Teal |
| Archived | `#7a726a` | Warm gray |
```

Replace with:

```markdown
### Status Colors (red/green-safe, grouped by lifecycle phase)

Statuses are distinguished primarily on the **blue↔yellow axis** (the axis red/green
color vision reads reliably) plus lightness, grouped by phase: shooting = cool blues,
development = warm ambers, finished = neutral (cool `scanned` vs warm `archived`). The
status label text is always shown alongside the color, so color is never the sole signal.

| Status | Hex | Phase |
|---|---|---|
| Loaded | `#6fbcff` | Shooting — vivid azure |
| Shooting | `#93c4ec` | Shooting — sky blue |
| Shot | `#afb3ea` | Shooting — blue-violet |
| At Lab | `#e3a347` | Development — amber |
| Lab Done | `#f0cf57` | Development — yellow |
| Developing | `#dd8b44` | Development — orange |
| Developed | `#ecc185` | Development — light tan |
| Scanned | `#a8cdd8` | Finished — cool slate |
| Archived | `#b3a99c` | Finished — warm taupe |
```

- [ ] **Step 3: Replace the Danger section**

Find (lines ~60–62):

```markdown
### Danger

Use `#c45c4a` for destructive actions (warm red, not pure red).
```

Replace with:

```markdown
### Danger

Destructive actions must read as dangerous without relying on red hue (the app
supports red/green color vision). Tokens: `--color-danger` (`#c0473a`, solid fill,
white text ~5:1), `--color-danger-hover` (`#d2594c`), `--color-danger-fg` (`#e88c80`,
danger icons/text on the dark surface). The `danger` Button variant is a solid fill,
and destructive commits carry a `Trash2` line icon; `ConfirmDialog` adds an
`AlertTriangle` in the header. Informational error banners may stay tinted (they carry
explanatory text and are not actions).
```

- [ ] **Step 4: Update the Button variant table row**

Find (line ~102):

```markdown
| `danger` | Red-tinted background |
```

Replace with:

```markdown
| `danger` | Solid red fill, white text (high-emphasis destructive); pair with a `Trash2` icon |
```

- [ ] **Step 5: Commit**

```bash
git add UI_DESIGN.md
git commit -m "docs(ui): document accessible palette, colorblind rationale, danger pattern"
```

---

### Task 7: Full verification

**Files:** none (verification only)

- [ ] **Step 1: Deterministic WCAG contrast assertion**

Run this from the repo root (asserts every text tier + status-as-text + white-on-danger ≥4.5:1 on the surface):

```bash
node -e '
const lum=(h)=>{const c=h.match(/\w\w/g).map(x=>parseInt(x,16)/255).map(v=>v<=.03928?v/12.92:((v+.055)/1.055)**2.4);return .2126*c[0]+.7152*c[1]+.0722*c[2]};
const cr=(a,b)=>{const L1=lum(a),L2=lum(b),hi=Math.max(L1,L2),lo=Math.min(L1,L2);return (hi+.05)/(lo+.05)};
const S="151210";
const checks=[["text","ece6dd",S],["muted","bcb0a0",S],["faint","9d9384",S],["loaded","6fbcff",S],["shooting","93c4ec",S],["shot","afb3ea",S],["at-lab","e3a347",S],["lab-done","f0cf57",S],["developing","dd8b44",S],["developed","ecc185",S],["scanned","a8cdd8",S],["archived","b3a99c",S],["white-on-danger","ffffff","c0473a"]];
let ok=true;for(const[n,fg,bg]of checks){const r=cr(fg,bg);const pass=r>=4.5;if(!pass)ok=false;console.log((pass?"PASS":"FAIL").padEnd(5),n.padEnd(16),r.toFixed(2)+":1")}
process.exit(ok?0:1)'
```

Expected: every line prints `PASS`, exit code 0.

- [ ] **Step 2: Type/build check**

Run: `cd frontend && bun run check && bun run build`
Expected: svelte-check `0 ERRORS 0 WARNINGS`; build `✓ built` / `done`.

- [ ] **Step 3: Visual sweep**

With `just dev`, walk through on `http://localhost:5173`: dashboard, rolls list (group-by-status), a roll detail (status chevron bar, badges, dim metadata now legible), the Stats status-distribution bar + legend, and one destructive confirm dialog (warning + trash icons, solid red). Confirm readability at normal brightness and that statuses are distinguishable.

- [ ] **Step 4: Final commit (if any docs/notes changed); otherwise push**

```bash
git status   # expect clean working tree (beads jsonl excluded)
```

---

## Self-review notes (author)

- **Spec coverage:** text tiers (Task 1) ✓, status palette (Task 2) ✓, danger tokens + button (Task 3) ✓, ConfirmDialog (Task 4) ✓, explicit Delete buttons (Task 5) ✓, UI_DESIGN.md incl. colorblind rationale (Task 6) ✓, contrast/build/visual verification (Task 7) ✓. Non-goals (surfaces, per-status icons, error banners, "liveliness" → kammerz-r35) intentionally untouched.
- **Type/utility consistency:** new tokens `--color-danger` / `--color-danger-hover` / `--color-danger-fg` are used exactly as `bg-danger` / `bg-danger-hover` / `text-danger-fg` and `bg-danger/15` — all valid Tailwind v4 token-derived utilities (mirrors existing `bg-accent` / `bg-accent-hover` usage). `Trash2` / `AlertTriangle` imported in every file that renders them.
- **No placeholders:** every edit shows exact find/replace content; verification steps give exact commands + expected output.
