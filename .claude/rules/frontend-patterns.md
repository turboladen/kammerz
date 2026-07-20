# Frontend Patterns

## Svelte 5 Patterns

- Use `$state()`, `$derived()`, `$effect()`, `$props()`, `$bindable()` — no legacy `let` reactivity.
- Use `onclick={handler}` on buttons instead of `<form onsubmit>`. (Historically a Tauri WebKit workaround; kept as a project convention for consistency across the existing pages.)
- Button component passes `onclick` via `{...rest}` spread to the native `<button>` element.
- Detail page edit mode: When a page has view/edit toggle, maintain parallel `$derived` vars — e.g., `selectedCamera` (from saved `roll.camera_id`) for shot defaults vs `editSelectedCamera` (from `editCameraId` form state) for edit-mode film stock/lens filtering.

## Component Patterns

- List pages use `ListToolbar` (search + group-by + sort) with `$bindable()` props. Pipeline: primary filter (ownership/status/type tabs) → `filterBySearch()` → sort → `groupItems()` — all via `$derived` chain. Utilities in `src/lib/utils/list.ts`.
- `GroupHeader` renders the ledger-line group label. Uses `{#if label}` guard so `groupBy === 'none'` (empty-string key) renders nothing.
- Collection Cards (cameras, lenses): `grid-cols-[repeat(auto-fill,minmax(260px,1fr))]` card grid for short scannable data. Lenses use `minmax(280px,1fr)` for edit/delete buttons.
- List Rows (rolls, film stocks): Full-width `px-4 py-2.5` rows with `gap-1.5` for items with wide relational data.
- `totalCount` in ListToolbar: Pass post-primary-filter count (e.g., `afterOwnerFilter.length`), not `items.length` — the "X of Y" denominator should reflect the active tab scope.
- Empty state on list pages: Three-branch pattern — `resultCount === 0 && items.length === 0` → `EmptyState` with icon + CTA; `resultCount === 0` → "No matches" text; else → render items.
- Dialog Cancel buttons must call `resetForm()` to clear stale form data (same as the success path). Wire `onclose={resetForm}` on every form `Dialog` so the X / Escape / backdrop close paths reset too. Do NOT fold the page-level `error=''` into `resetForm()` — that wipes a still-relevant delete-failure banner when an Add dialog merely closes; clear `error` on dialog open/submit instead (kammerz-4zp).
- `ComboInput` dropdown options use `onmousedown` (not `onclick`) to beat the blur/click race condition. The label is wired via `<label for={id}>` + `id` on the input, with full combobox ARIA (role/aria-expanded/aria-activedescendant) — match that when editing it (kammerz-dsy).
- `Select` options support an optional `disabled` property (used for visual dividers like `── Other formats ──`).
- `Select` uses explicit `h-[38px]` to match `Input` height — WebKit renders `<select>` shorter than `<input>` with identical padding classes. Date and time entry: dates use the native `<Input type="date">` (full `YYYY-MM-DD` only — ADR-0011); time uses the custom `TimeInput` (24-hour `HH:MM` text field — ADR-0010, since native `<input type="time">` is locale-12h). Add `class="h-[38px]"` to date inputs for row parity.
- Use `$derived.by(() => { ... })` when derived state needs multi-line logic; `$derived(expr)` for one-liners.
- Use `{#snippet name(params)}` / `{@render name(params)}` for reusable template blocks within a single component — avoids duplicating markup without extracting a separate component file. See `rollCard` snippet in `+page.svelte` (Dashboard).
- Always use the `<Badge badge={roll.badge} groupKey={roll.group_key} />` component for a roll's lifecycle pill — never inline status pills with raw classes. Badge takes the server-derived compound `badge` label and colors the pill by `group_key` phase via `phaseTheme` (ADR-0013). Search hits and dev-list rows carry the same `badge`/`group_key` fields for this.
- Wrap page content sections in `<FadeIn>` with staggered `delay` props (typically 50ms increments) for consistent entrance animations.
- FadeIn stacking context: CSS `animation` with `transform`/`opacity` creates a stacking context + containing block, trapping `position: fixed` children (e.g., Dialogs). FadeIn strips its animation class via `onanimationend` to clear this after the entrance plays. Never wrap a component that renders its own Dialog inside a persistent animation/transform.
- Section headers use the ledger-line pattern: `text-xs font-semibold uppercase tracking-wider text-text-faint` with either a rule line (`<div class="flex-1 border-b border-border-subtle">`) or `justify-between` for headers with action buttons. Never use `text-sm font-semibold text-text-muted`.
- Card hover borders always use `hover:border-accent/40` — never other opacities like `/30`.
- **Lifecycle display is phase-based (ADR-0013).** There is no `RollStatus` enum and no `status.ts` — a roll carries server-derived `activities`/`badge`/`group_key`/`done` (see `src/activity.rs`), and the frontend never re-derives. Phase display metadata (label, color, pill/dot classes per `group_key` 0..=5) lives in `src/lib/utils/phase.ts` `PHASE_META`; use `phaseTheme(groupKey)` for the Badge and `phaseLabel(groupKey)` for group-by/filter/stats labels. **Phase labels exist ONLY in `PHASE_META`** — the backend speaks `group_key` integers on every surface (including the stats `rolls_by_phase` buckets, which are `{group_key, count}`), so there is no cross-language label-string contract to drift. Never add a backend label list; if a surface seems to need one, return `group_key` instead. The rolls list filters/groups by `group_key`; the dashboard sections and pipeline bar bucket by `group_key`/`done`. The roll page renders an activity board (`ActivityBoard.svelte`) driven by the same fields; recording a dev record or lifecycle date is the state change, and backward moves (date-clears) go through `ConfirmDialog`. The old chevron control, status flows, `status-flows.json`, and `tests/status_flows.rs` are all gone — there is no fixture cross-check; the derivation's coverage is `src/activity.rs`'s unit tests.
- Lens naming: Lenses use `brand` + `model` (paralleling cameras). The `model` field (formerly `name_on_lens`) should NOT include the brand. `lensDisplayName()` in `$lib/utils/lens.ts` always returns `brand + model`, with a `startsWith` guard to avoid doubling when model already contains the brand. Fallback when model is empty: `brand + focal_length + max_aperture`.
- Lens dropdowns: Always use `buildLensOptions()` from `$lib/utils/lens.ts` — handles mount-compatibility sorting with dividers and automatic disambiguation of duplicate lenses. Also see `buildMountOptions()` for mount dropdowns grouped by format family (it carries the inline `+ New mount…` sentinel — kammerz-snh).
- Camera dropdowns: Use `buildCameraLabels()` from `$lib/utils/disambiguate.ts` to get a `Map<id, label>` that auto-disambiguates duplicate cameras. When two cameras share the same `brand + model`, the label appends `(S/N xxxxx)` if serial exists or `(Copy N)` by creation order. Single instances stay clean.
- Dialog component uses flex column layout with `max-h-[85vh]` and `overflow-y-auto` on content. When adding fields to dialogs (e.g., inline lens creation), scrolling is already handled.
- Shot **aperture and shutter_speed are stored BARE** — aperture `2.8` (never `f/2.8`), shutter `1/125` or `4` (no trailing `s`). Every display site prepends `f/` / appends `s` (FrameStrip, `rolls/[id]/print`, etc.) and inputs supply the `f/` label. Migration `m…025_normalize_aperture_bare` normalized legacy `f/`-prefixed rows — writing an `f/`-prefixed value double-prefixes to `f/f/2.8` (kammerz-jd1).

## Error Handling

- Frontend `request()` calls (and the `src/lib/api/` wrappers over them) reject with `ApiRequestError` on a non-2xx response, carrying the backend `{error: {code, message}}`. Wrap in try/catch with user-visible error display. A 401 fires the registered unauthorized handler (redirect to `/login`).
- Always validate required fields client-side before API calls (brand, model, mount, etc.) on BOTH add and edit/save paths. Show inline `error` state text — don't rely on backend DB constraint errors which are opaque to users. (The server also validates now — see `validate.rs` in the backend rules — but the client guard gives the better message.)

## UI Design

- Follow the design system in `UI_DESIGN.md` — colors, typography, component styling, layout patterns, and design principles.
- All colors use CSS custom properties defined in `frontend/src/app.css` via Tailwind's `@theme`. Never use raw hex colors.
- Fonts: DM Sans (UI), IBM Plex Mono (data), Instrument Serif (display). Self-hosted in `frontend/static/fonts/`.
- Keep `UI_DESIGN.md` updated when design decisions change.
