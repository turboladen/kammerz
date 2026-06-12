# Accessible theme: contrast + red/green-safe palette

**Date:** 2026-06-02
**Status:** Implemented (see revision below)

> **Revision 2026-06-03 — palette pivoted to graphite/silver.** Real-app review
> showed the original plan's decision to _keep the warm-brown surfaces_ was the
> core problem: the brown wash overwhelmed text and the status chevron bar stayed
> brown-on-brown. The implemented theme instead re-bases to a **near-neutral
> graphite/black** surface (`#0d0d0c` / `#161615` / `#211f1e`) with **brushed-silver
> text** (`#eeebe7` / `#bab4ad` / `#8f8981`) and keeps **amber** (`#e2a45e`) as the
> single warm accent. Refinement that the warm theme got from color now comes from
> **material + texture** instead (card top catch-light + sheen + elevation, film
> grain at 5%, a subtle vignette) — none of it behind text, so contrast holds. The
> red/green-safe status palette and the danger treatment below are unchanged and
> carried over. The chevron bar was fixed (done = amber on neutral, current = solid
> amber, future = readable silver), and reversible confirms (backward status moves)
> use a non-destructive `primary` variant. Final values are documented in
> `UI_DESIGN.md`. Sections below describe the original (superseded) warm-surface
> approach; read them as history.

## Problem

The "Darkroom Ledger" theme is hard to read without turning screen brightness
up, and its status colors are difficult to distinguish for a user with red/green
(deutan/protan) color vision deficiency.

Two concrete defects:

1. **Contrast.** `--color-text-faint` (`#6d6258`) sits at ~3.2:1 on the
   near-black surface (`#151210`) — below WCAG AA (4.5:1) for normal text. It is
   used pervasively (section headers, dates, hints, metadata, empty-state text),
   so a large fraction of secondary text is effectively unreadable without
   cranking brightness.
2. **Status hues.** The 9 status colors collapse for red/green vision (and some
   even for full-color vision): `shooting` green vs `scanned` teal are nearly
   identical; `shot` and `developed` are nearly the same purple; the warm
   cluster (`at-lab`/`lab-done`/`developing`) blends together and with the amber
   accent. Encoding status on a green↔red axis is the worst case for this
   deficiency.
3. **Danger affordance (added requirement).** Destructive actions use a faint
   red tint (`bg-red-500/15 text-red-400`) that, for red/green vision, reads as
   just another muted warm button rather than "dangerous."

## Goals

- Every text tier clears WCAG AA (≥4.5:1) on the surface.
- Status colors are distinguishable for red/green vision, primarily via the
  blue↔yellow axis (which that deficiency reads reliably) plus lightness.
- Destructive actions are unmistakably dangerous **without relying on red hue**
  — carried by emphasis, an icon, and the existing confirm step.
- The "Darkroom Ledger" identity is preserved: warm near-black surfaces, the
  amber "safelight" accent, the film-grain texture, and the
  serif/mono/sans type system are unchanged.

## Non-goals (explicitly out of scope)

- **Lightening the surfaces / a light or high-contrast toggle.** Chosen
  direction keeps the near-black darkroom surface; we fix contrast by lifting
  the text tiers, not the canvas.
- **Per-status icons/shapes.** Every status already shows its text label
  everywhere it renders (badges, the chevron bar, list rows), so color is
  reinforcement, not the sole signal — which satisfies the accessibility
  requirement. Per-status iconography is a possible future enhancement, not part
  of this work.
- **General UI "liveliness"/visual interest.** Tracked separately as
  `kammerz-r35`; it must build on this accessible base, not undo it.
- **Recoloring informational error banners** (`bg-red-500/15 text-red-400`).
  Those carry explanatory text and are not destructive actions; left as-is.

## Design

### Architecture / why this is small

Every component sources its colors from CSS custom properties declared in the
`@theme` block of `frontend/src/app.css`. `statusConfig` in
`frontend/src/lib/utils/status.ts` references `var(--color-status-*)`, and
badges/dots/chevrons use the generated Tailwind `*-status-*` utility classes.
Therefore the palette change is **value swaps in one `@theme` block**, and it
propagates everywhere automatically. No TypeScript/Svelte logic changes for the
palette. The danger change touches two shared components.

### 1. Text tiers (contrast fix)

In `app.css` `@theme`:

| token                | current   | new       | approx contrast on `#151210` |
| -------------------- | --------- | --------- | ---------------------------- |
| `--color-text`       | `#e8e2d9` | `#ece6dd` | ~15:1                        |
| `--color-text-muted` | `#a09488` | `#bcb0a0` | ~8.8:1                       |
| `--color-text-faint` | `#6d6258` | `#9d9384` | ~6.2:1                       |

Surfaces, borders, accent, fonts, and the grain texture are unchanged.

### 2. Status palette (red/green-safe)

Grouped by lifecycle phase — **blue → amber → neutral** — with lightness
separating statuses within a phase. The exact status is always also spelled out
in its text label.

| status     | current   | new       |
| ---------- | --------- | --------- |
| loaded     | `#6ca4d4` | `#6fbcff` |
| shooting   | `#5cb88a` | `#93c4ec` |
| shot       | `#a38bc7` | `#afb3ea` |
| at-lab     | `#d4a84e` | `#e3a347` |
| lab-done   | `#d4c24e` | `#f0cf57` |
| developing | `#c87a42` | `#dd8b44` |
| developed  | `#b384c7` | `#ecc185` |
| scanned    | `#5aaf9e` | `#a8cdd8` |
| archived   | `#7a726a` | `#b3a99c` |

Notes:

- Shooting phase = cool blues (loaded vivid azure, shooting paler blue, shot
  blue-violet).
- Development phase = warm ambers (lab path `at-lab`/`lab-done`; self path
  `developing`/`developed`; the two paths rarely co-occur on one roll).
- Finished phase = neutral, split cool (`scanned`) vs warm (`archived`) so they
  separate on the blue↔yellow axis.
- Each new status color, used as badge **text** on the surface, must stay
  ≥4.5:1 (the brightened values were chosen to satisfy this).

### 3. Danger affordance

Keep the red hue but stop depending on it. Introduce a danger fill token and
apply a high-emphasis treatment to destructive **commits**:

- Add to `@theme`: `--color-danger: #c0473a` (solid fill; white text on it is
  ~5:1, clears AA). The on-danger text is white.
- `frontend/src/lib/components/ui/Button.svelte`, `danger` variant: change from
  the faint tint to a **solid fill** (`--color-danger` background, white text,
  semibold) so it reads as a serious action by weight/emphasis. Support an
  optional leading icon.
- Explicit "Delete" buttons render a leading **`Trash2`** line icon
  (`lucide-svelte`, already a dependency) as a white stroke.
- `frontend/src/lib/components/ui/ConfirmDialog.svelte`: add a leading
  **`AlertTriangle`** icon (in the danger tint) in the header for the `danger`
  variant, and the confirm button uses the solid danger + `Trash2` icon.
- Quiet inline "×" remove buttons are unchanged — they only open the confirm
  dialog, which now carries the strong danger treatment.

### Components touched

- `frontend/src/app.css` — `@theme`: 3 text tiers, 9 status colors, 1 danger token.
- `frontend/src/lib/components/ui/Button.svelte` — `danger` variant styling +
  optional icon support.
- `frontend/src/lib/components/ui/ConfirmDialog.svelte` — warning icon + iconed
  confirm.
- Explicit destructive "Delete" buttons across pages — add the `Trash2` icon
  (audit usages of `variant="danger"`).
- `UI_DESIGN.md` — document the new palette, the colorblind rationale, and the
  danger pattern. Update the danger color reference (currently `#c45c4a`) to the
  new `--color-danger`.

## Accessibility acceptance

- All three text tiers ≥4.5:1 on `--color-surface`.
- Each status color as badge text ≥4.5:1 on `--color-surface`.
- White text on `--color-danger` ≥4.5:1.
- Status remains distinguishable with a text label present in every location it
  renders (no color-only status anywhere).

## Testing

- `bun run check` (svelte-check) and `bun run build` must pass.
- Visual verification via `just dev` (and/or Playwright): roll detail page
  (status badges, chevron bar, dim metadata/section labels now legible),
  rolls/cameras/lenses list rows, the stats status-distribution chart + legend,
  and a delete confirm dialog (danger treatment + trash icon).
- Spot-check contrast ratios for the new tiers and the danger fill.
- No backend changes; no existing tests should be affected (pure CSS + markup).

## Risks / notes

- `statusConfig.pillClasses` use `bg-status-*/10 text-status-*`; the `/10`
  tinted background + full-color text rendering is preserved — only the
  underlying hex changes.
- The stats page status-distribution chart keys off the same status vars and a
  legend; verify it still reads after the swap (expected: yes).
- This branch is based on `main` and is independent of PR #16
  (`kammerz-a89-3ox-status-workflow`); the files do not overlap, so no conflict
  is expected. Rebase after #16 merges if desired.
