# Guided f/ and shutter inputs (kammerz-dzuy)

**Status:** Design. No current ADR — this is a feature, not a cross-cutting architectural decision.

## Context

Shot **aperture** (`f/`) and **shutter speed** are entered as free-text inputs in two places: the `QuickAddBar` compact-entry card and the full Shot add/edit dialog in `rolls/[id]/+page.svelte`. Free text invites typos — `56` instead of `5.6`, `f/5.6` (which double-prefixes to `f/f/2.8` at display sites per kammerz-jd1), `1/250s`, `5,6`. Both values are almost always drawn from a small set of standard stops, yet some cameras legitimately use non-standard values: half/third-stop detents, continuously-variable apertures on early lenses, and odd legacy/large-format shutter speeds.

**Goal:** steer entry toward standard values to cut typos, while never blocking a legitimate special case. Reuse the existing `ComboInput` (a free-text combobox with a filtering suggestions dropdown). Values must persist **bare** — aperture `5.6` (never `f/5.6`), shutter `1/250` (never `1/250s`) — because every display site prepends `f/` / appends `s`.

**Non-goals:** backend format validation (the API stays permissive so special cases go through); a new bespoke picker widget; changing storage format or the DB schema.

## Design

### Part 1 — `frontend/src/lib/utils/exposure.ts` (new)

Pure, unit-testable constants + functions (fits the `$lib/utils` + vitest-gate convention). The **suggestion** lists and the **recognized** sets are intentionally decoupled: suggestions stay concise; recognition is broad so the advisory hint only fires on genuine typos.

- `APERTURE_SUGGESTIONS: string[]` — the **half-stop** dropdown list:
  `1, 1.2, 1.4, 1.7, 2, 2.4, 2.8, 3.4, 4, 4.8, 5.6, 6.7, 8, 9.5, 11, 13, 16, 19, 22, 27, 32`
- `SHUTTER_SUGGESTIONS: string[]` — standard speeds, fast→slow, then seconds, then bulb:
  `1/4000, 1/2000, 1/1000, 1/500, 1/250, 1/125, 1/60, 1/30, 1/15, 1/8, 1/4, 1/2, 1, 2, 4, 8, 15, 30, B`
- `isRecognizedAperture(v: string): boolean` — membership in a **broad** recognized set = union of full ∪ half ∪ third stops from ~f/1 to f/64 (so `1.8`, `2.5`, `3.5`, `45`, `64` are recognized; `56`, `8.5`, `f/25` are not). Exact array defined in the util.
- `isRecognizedShutter(v: string): boolean` — the standard speeds **plus** common legacy/old-sequence speeds (e.g. `1/25`, `1/50`, `1/100`, `1/200`, `1/300`, `1/400`) and long exposures, so old-camera and LF values don't nag. Exact array in the util.
- `normalizeAperture(v: string): string` — strip a leading `f/` or `f`, convert comma→dot, trim, collapse internal whitespace → bare number string. Never guesses digits, never rejects.
- `normalizeShutter(v: string): string` — strip a trailing `s`, trim/collapse whitespace; preserve `1/x`, integer seconds, and `B`/`T` (bulb/time) untouched.

Both `normalize*` are idempotent and return `''` for empty input.

### Part 2 — `ComboInput` gains two optional, domain-agnostic props

`ComboInput` currently takes `label / hint / placeholder / value / options`. Add:

- `normalize?: (v: string) => string` — invoked inside the existing `handleBlur` (after its 150ms option-click delay) and on option-select, mutating the committed `value`. This enforces bare storage at the source.
- `warning?: string` — a warning-styled line (amber, distinct from the neutral `hint`), rendered **only when the field is not focused** (gated on the internal `showDropdown` state) so it never flickers while typing.

No domain logic enters `ComboInput` — it stays generic and reusable for its existing Brand / Purchased-From uses.

### Part 3 — Wiring

Replace the four plain inputs with `ComboInput`, keeping the same bound vars and compact widths:

- `QuickAddBar.svelte` — `aperture`, `shutterSpeed`
- `rolls/[id]/+page.svelte` Shot add/edit dialog — `shotAperture`, `shotShutterSpeed`

Each consumer passes, e.g. for aperture:

```
options={APERTURE_SUGGESTIONS}
normalize={normalizeAperture}
warning={aperture && !isRecognizedAperture(aperture) ? 'Non-standard f/ value' : ''}
```

Analogous for shutter with the shutter helpers.

**Belt-and-suspenders:** the save handlers (`handleSave` in `QuickAddBar`, the create/update paths in the Shot dialog) run `normalize*` when building the payload, so a keyboard save (⌘/Ctrl+Enter, which fires before blur) still stores clean bare values.

### Backend

Unchanged. `src/routes/shots.rs` keeps aperture/shutter as permissive `Option<String>` with `trim_opt`. No format validation — special cases must persist. The `f/`-double-prefix trap is covered by the frontend `normalize*` plus the existing `m…025_normalize_aperture_bare` migration.

## Testing

- **vitest** (`frontend/src/lib/utils/exposure.test.ts`) — the durable gate. Cover `normalizeAperture`/`normalizeShutter` (`f/5.6`→`5.6`, `F5.6`→`5.6`, `5,6`→`5.6`, `1/250s`→`1/250`, `B`→`B`, `''`→`''`, idempotency) and `isRecognized*` (recognized: `5.6`, `1.8`, `3.5`, `64`, `1/50`; off-list: `56`, `8.5`, `250` (bare — missing the `1/`), `1/275`). Runs in `ci-frontend`.
- **Component behavior** — manual / Playwright per the project's component convention (components/routes are e2e-only): pick a suggestion, free-type a special value, confirm the ⚠ appears only after blur for a genuine off-list value and not for `1.8`, confirm bare storage via a shot round-trip.
- No backend tests (nothing changed there).

## Files

- **New:** `frontend/src/lib/utils/exposure.ts`, `frontend/src/lib/utils/exposure.test.ts`
- **Edit:** `frontend/src/lib/components/ui/ComboInput.svelte` (add `normalize`, `warning`), `frontend/src/lib/components/rolls/QuickAddBar.svelte`, `frontend/src/routes/(app)/rolls/[id]/+page.svelte`
- **Unchanged:** backend, DB schema, migrations
