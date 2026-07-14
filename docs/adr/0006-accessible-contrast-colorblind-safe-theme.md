# ADR-0006: Accessible contrast + colorblind-safe theme tokens

- **Status:** Accepted
- **Date:** 2026-06-02
- **Related:** `UI_DESIGN.md` (Color Palette, Danger sections), `frontend/src/app.css` (`@theme` block), `frontend/src/lib/utils/status.ts`, `frontend/src/lib/components/ui/Button.svelte`, `frontend/src/lib/components/ui/ConfirmDialog.svelte`

## Context

The "Darkroom Ledger" dark theme had two accessibility defects: `--color-text-faint`
sat at ~3.2:1 on the near-black surface (below WCAG AA), and it was used
pervasively (section headers, dates, hints), so a large share of secondary text
was effectively unreadable without cranking brightness. Separately, the 9 roll
status colors and the destructive-action affordance both leaned on hues
(green/red, warm-cluster blends) that collapse for red/green (deutan/protan)
color vision deficiency — the worst-case axis to encode meaning on.

## Decision

Fix accessibility by adjusting **values within the existing token system**, not
by changing the theme's identity (near-black surfaces, amber accent, serif/mono/
sans type stay put — see `UI_DESIGN.md` Design Principles). Every component
sources color from CSS custom properties in `frontend/src/app.css`'s `@theme`
block, so this is a small, propagating value swap rather than a component rewrite:

- **Text tiers:** all three (`--color-text`, `--color-text-muted`,
  `--color-text-faint`) must clear WCAG AA (≥4.5:1) on `--color-surface`. Any new
  text tier or surface added later must be checked against this bar before
  merging.
- **Status colors:** the 9 lifecycle statuses are grouped by phase — shooting
  (cool blues) → development (warm ambers) → finished (neutral, cool vs warm) —
  and separated by lightness within a phase, so the distinguishing signal rides
  the blue↔yellow axis (reliable for red/green deficiency) rather than
  green-vs-red hue. Color is reinforcement only: the status label text is always
  rendered alongside the color (badges, chevron bar, list rows) — never a
  color-only signal. Each status color, used as text on the surface, must itself
  clear ≥4.5:1.
- **Danger affordance:** destructive commits don't rely on red hue alone. The
  `danger` Button variant is a solid fill (`--color-danger`) with white text and a
  leading `Trash2` icon; `ConfirmDialog`'s danger variant adds an `AlertTriangle`
  header icon. Reversible-but-confirmed actions (e.g. moving a roll status
  backward) use `ConfirmDialog variant="primary"` instead, so the strong danger
  treatment stays reserved for true data loss. Informational error banners are
  exempt (they carry explanatory text, aren't actions).
- Surfaces are intentionally _not_ lightened and there is no light/high-contrast
  theme toggle — contrast is fixed by lifting text, not the canvas.

## Consequences

- Any future token addition (new status, new text tier, new semantic color) must
  be checked for both AA contrast and blue↔yellow-axis distinguishability before
  merging — this is now a standing design constraint, not a one-time pass.
- Status meaning must never be color-only; adding a new status requires a text
  label everywhere it renders, or the accessibility guarantee breaks silently.
- The "liveliness"/decorative-motif work (film grain, sprockets, frame counters)
  is built on top of this base and is constrained to never sit behind text or
  touch a color token, so it can't regress contrast.
- Cost: every new component that introduces color must route through the
  `@theme` tokens rather than ad hoc hex/Tailwind palette classes, or it escapes
  both the contrast and colorblind guarantees.
