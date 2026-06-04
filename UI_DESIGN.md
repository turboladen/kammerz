# UI Design System: "The Darkroom Ledger"

Kammerz's visual identity draws from two analog photography metaphors:

1. **The darkroom** — deep graphite blacks lit by a single amber safelight; brushed-metal camera bodies, atmospheric film grain
2. **The field log** — photographer's notebook, precise frame numbers, monospaced data, ledger-style precision

The palette is intentionally near-monochrome — black/graphite surfaces and brushed-silver text — so the one warm accent (amber) and the status colors carry all the meaning. It's built for **readability** (every text tier clears WCAG AA) and for **red/green color vision** (status is distinguished on the blue↔yellow axis plus lightness, never by hue alone). Refinement comes from material and texture — a top catch-light and soft elevation on cards, film grain, and a subtle vignette — not from a warm color wash (which is what crushed contrast in the earlier brown theme).

---

## Color Palette

All colors are defined as CSS custom properties in `frontend/src/app.css` via Tailwind CSS 4's `@theme` block. Every component references these tokens — changing the theme transforms the entire app.

### Surfaces (graphite / black — a camera body, not warm brown)

Near-neutral with only a whisper of warmth. Dark enough to hold contrast; the warmth is mood, the brightness lives in the text.

| Token | Hex | Usage |
|---|---|---|
| `--color-surface` | `#0d0d0c` | Page background, input backgrounds |
| `--color-surface-raised` | `#161615` | Cards, sidebar, dialog panels |
| `--color-surface-overlay` | `#211f1e` | Dropdown menus, hover states, chips |

### Borders (graphite grays)

| Token | Hex | Usage |
|---|---|---|
| `--color-border` | `#2f2d2a` | Card borders, input borders, dividers |
| `--color-border-subtle` | `#242220` | Sidebar border, softer dividers |

### Text (brushed silver — bright, faint warm lean)

| Token | Hex | Usage |
|---|---|---|
| `--color-text` | `#eeebe7` | Primary text, headings (~16:1 on surface) |
| `--color-text-muted` | `#bab4ad` | Secondary text, labels, descriptions (~9:1) |
| `--color-text-faint` | `#8f8981` | Tertiary text, hints, timestamps (~5.6:1 — all tiers clear WCAG AA) |

### Accent (amber/gold — the single warm "safelight" spark)

The one warm element against the neutral field. Used deliberately for primary actions, active states, and the brand — not as a pervasive wash.

| Token | Hex | Usage |
|---|---|---|
| `--color-accent` | `#e2a45e` | Primary buttons, active states, brand |
| `--color-accent-hover` | `#eeb878` | Button hover states |
| `--color-accent-muted` | `#7d5a32` | Subtle accent backgrounds |

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

### Danger

Destructive actions must read as dangerous without relying on red hue (the app
supports red/green color vision). Tokens: `--color-danger` (`#c0473a`, solid fill,
white text ~5:1), `--color-danger-hover` (`#b03f33`, a *darker* hover so white text
stays ≥4.5:1 — a lighter hover would drop below AA), `--color-danger-fg` (`#e88c80`,
danger icons/text on the dark surface). The `danger` Button variant is a solid fill,
and destructive commits carry a `Trash2` line icon; `ConfirmDialog` adds an
`AlertTriangle` in the header. Informational error banners may stay tinted (they carry
explanatory text and are not actions). For **reversible** actions that need confirmation
but aren't data loss (e.g. moving a roll's status backward), use `ConfirmDialog
variant="primary"` — an amber confirm with no warning icon or trash — so the destructive
styling stays reserved for true deletes.

---

## Typography

Fonts are self-hosted in `frontend/static/fonts/` and loaded via `@font-face` declarations in `frontend/src/app.css`.

### Font Stack

| Role | Font | Weights | CSS Variable |
|---|---|---|---|
| **UI (sans)** | DM Sans | 400, 500, 600 | `--font-sans` |
| **Data (mono)** | IBM Plex Mono | 400, 500 | `--font-mono` |
| **Display (serif)** | Instrument Serif | 400 | `--font-display` |

### Usage Guidelines

- **DM Sans** — All body text, labels, buttons, descriptions. Geometric with humanist warmth; evokes camera body markings.
- **IBM Plex Mono** — Roll IDs, serial numbers, ISO values, aperture values, stat numbers. Industrial/mechanical; evokes stamped serial numbers and frame counters.
- **Instrument Serif** — Brand name ("Kammerz"), page titles, dialog titles. Editorial quality, used sparingly for warmth. Applied via `.font-display` utility class or Tailwind's `font-display`.

### Never Use

- Inter, Roboto, Arial, system-ui as primary fonts
- Instrument Serif for body text or small labels

---

## Components

### Button (`frontend/src/lib/components/ui/Button.svelte`)

Four variants, two sizes:

| Variant | Appearance |
|---|---|
| `primary` | Solid amber accent background |
| `secondary` | Bordered, overlay background |
| `ghost` | Text-only, overlay on hover |
| `danger` | Solid red fill, white text (high-emphasis destructive); pair with a `Trash2` icon |

Sizes: `md` (default, px-4 py-2) and `sm` (px-2.5 py-1.5).

### Badge (`frontend/src/lib/components/ui/Badge.svelte`)

Roll status pills with a small color dot indicator:
- 1.5px solid dot before the label text
- Background at 10% opacity of the status color
- Text in the status color
- `rounded-full` shape
- Always use `<Badge>` for roll statuses — never inline status pills.

### Dialog (`frontend/src/lib/components/ui/Dialog.svelte`)

- Backdrop: `bg-black/50` with `backdrop-blur-sm`
- Panel: `rounded-lg`, `shadow-2xl`, `border-border`
- Title: `font-display text-xl` (Instrument Serif)
- Entry animation: 150ms scale from 0.95 + fade
- Backdrop animation: 100ms fade

### ConfirmDialog (`frontend/src/lib/components/ui/ConfirmDialog.svelte`)

Same animations and styling as Dialog, smaller max-width (`max-w-sm`).

### Input, Select, Textarea, ComboInput

- Full border style: `border-border bg-surface`
- Focus: `border-accent ring-1 ring-accent/50`
- Labels: `text-xs font-medium text-text-muted`
- Hints: `text-xs text-text-faint`

### EmptyState (`frontend/src/lib/components/ui/EmptyState.svelte`)

Centered message with optional CTA button. Used when lists are empty.

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

### FadeIn (`frontend/src/lib/components/ui/FadeIn.svelte`)

Wraps content in staggered `fade-in-up` animation (200ms, ease-out). Use on all page sections for sequential reveal:
- `delay` prop for staggering (typically 50ms increments between sections)
- Every page should wrap its main content sections in `FadeIn` for consistent entrance animations

---

## Layout

### App Shell (`frontend/src/routes/+layout.svelte`)

```
┌──────────────────────────────────────────┐
│ Sidebar (w-56)  │  Main Content (flex-1)  │
│                 │  ┌─────────────────────┐ │
│  Brand area     │  │ PageHeader          │ │
│  Navigation     │  ├─────────────────────┤ │
│  Quick Entry    │  │ Content (p-6)       │ │
│                 │  │ (scrollable)        │ │
│                 │  └─────────────────────┘ │
└──────────────────────────────────────────┘
```

- `flex h-screen overflow-hidden`
- Sidebar is fixed width, main content scrolls independently

### Sidebar (`frontend/src/lib/components/layout/Sidebar.svelte`)

- **Brand**: "Kammerz" in Instrument Serif (amber), "film log" subtitle in IBM Plex Mono (uppercase, tracked)
- **Background**: Gradient from `surface-raised` to `surface` (top to bottom)
- **Border**: `border-subtle` (softer than regular border)
- **Icons**: Lucide icons at 16px, `strokeWidth={1.75}`
- **Active state**: 2px left border in accent + `bg-accent/8` + text in accent
- **Hover state**: `bg-surface-overlay` + text lightens
- **Quick Entry**: Dashed border, transitions to solid accent on hover

Navigation is split into two groups with a subtle separator:

**Core navigation** (data entities):

| Route | Label | Icon |
|---|---|---|
| `/` | Dashboard | `LayoutDashboard` |
| `/rolls` | Rolls | `Film` |
| `/cameras` | Cameras | `Camera` |
| `/lenses` | Lenses | `Aperture` |
| `/film-stocks` | Film Stocks | `Package` |
| `/labs` | Labs | `FlaskConical` |
| `/developments` | Developing | `TestTube2` |

**Utility navigation** (separated by `border-t border-border-subtle`):

| Route | Label | Icon |
|---|---|---|
| `/search` | Search | `Search` |
| `/stats` | Stats | `BarChart3` |

### PageHeader (`frontend/src/lib/components/layout/PageHeader.svelte`)

- Title in `font-display text-xl`
- Description in `text-sm text-text-faint`
- Border: `border-border-subtle`
- Back link: `text-xs text-text-muted` with arrow, accent on hover
- Action buttons slot on the right

---

## Section Headers (Ledger-Line Pattern)

All section headers use a consistent ledger-line pattern that evokes ruled notebook pages:

```
text-xs font-semibold uppercase tracking-wider text-text-faint
```

With an extending rule line:
```svelte
<h2 class="mb-3 flex items-center gap-3 text-xs font-semibold uppercase tracking-wider text-text-faint">
    Section Title
    <div class="flex-1 border-b border-border-subtle"></div>
</h2>
```

When a section header has action buttons on the right (e.g., "+ Add"), use `justify-between` instead of the rule line:
```svelte
<div class="mb-3 flex items-center justify-between">
    <h2 class="text-xs font-semibold uppercase tracking-wider text-text-faint">Section Title</h2>
    <Button size="sm">+ Add</Button>
</div>
```

**Do not use** the old pattern: `text-sm font-semibold text-text-muted`.

---

## Card Patterns

### Collection Cards (cameras, lenses)

Multi-column card grid for equipment with short, scannable identity data:

```
grid grid-cols-[repeat(auto-fill,minmax(260px,1fr))] gap-2.5
```

Individual card:
```
rounded-lg border border-border bg-surface-raised px-3.5 py-3
transition-all duration-150
hover:border-accent/40 hover:-translate-y-px
```

- `h-full flex flex-col` on each card for equal heights within a row
- Primary line: Brand + Model (`text-sm font-semibold leading-snug`)
- Metadata line: specs joined by `·` separators (`text-xs text-text-muted`)
- Optional tertiary line: serial number, sold badge (`text-[11px] text-text-faint`)
- Prefix badges use `text-[10px]` for compactness
- Lenses use `minmax(280px, 1fr)` (slightly wider to accommodate edit/delete buttons)
- Lenses show edit/delete on group-hover at `opacity-0 → opacity-100`

### List Rows (rolls, film stocks, labs, search results)

Full-width rows for items with horizontal relational data:

```
rounded-lg border border-border bg-surface-raised px-4 py-2.5
transition-all duration-150
hover:border-accent/40 hover:-translate-y-px
```

- Row gap: `gap-1.5` (tighter than cards)
- `rounded-lg` (8px) — precise, tool-like
- Hover: border glows amber at `/40` opacity, subtle 1px lift
- Group-hover reveals edit/delete actions at `opacity-0 → opacity-100`

### Detail Cards (camera detail, roll detail)

```
rounded-lg border border-border bg-surface-raised p-5
```

No hover effect on detail cards — they're static content panels.

### Active Roll Cards (dashboard "In the Field")

```
rounded-lg border border-accent/20 bg-accent/5 p-4
hover:border-accent/40 hover:-translate-y-px
```

Warm amber tint to highlight actively shooting rolls.

### Compact Table Inputs (Import shots table)

For dense data entry tables, use smaller inputs with `rounded` (4px) instead of `rounded-lg`:
```
rounded border border-border bg-surface px-1.5 py-1 text-xs
focus:border-accent focus:ring-1 focus:ring-accent/50 focus:outline-none
```

This is a sanctioned variant for table cells where standard Input components would be too large.

---

## Page Designs

### Dashboard (`/`)

1. **"In the Field"** — Active rolls (loaded/shooting) as prominent amber-tinted cards
2. **Quick Stats** — 4-column grid: Total Rolls, Cameras, Currently Shooting, At Lab (all `font-mono text-2xl font-semibold`)
3. **Roll Pipeline** — Horizontal status distribution bar with proportional per-status-color segments + legend
4. **Needs Attention** — Rolls missing camera assignment or waiting at lab, with icon indicators

Empty state: Camera icon in accent circle, "Start your log" in Instrument Serif, explanatory text, CTAs.

### Search (`/search`)

- Debounced search input (300ms) with search icon prefix, autofocus
- Results grouped by entity type, each section with:
  - `border-l-2 border-accent/40` left accent for visual grouping
  - Entity icon + uppercase category header with count
  - List cards showing matched item + "in {match_field}" hint
  - Staggered `FadeIn` per category (50ms increments)

### Stats (`/stats`)

- **Summary Cards** — 4-column grid (Total Rolls, Total Shots, Total Costs, Cameras), all `font-mono text-2xl font-semibold`
- **Cost Breakdown** — Stacked bar (Lab Development amber, Maintenance muted) with legend
- **Rolls Per Month** — Horizontal bar chart (`bg-accent/80`, `rounded-r`)
- **Rankings Row** — 3-column grid: Top Film Stocks, Top Cameras, Top Lenses (numbered lists with accent count pills)
- **Distribution Row** — 3-column grid: Rolls by Format, Rolls by Lens Mount, Rolls by Status
  - Format and Mount charts use `bg-accent/80`
  - Status chart uses per-status CSS variable colors (matching Dashboard Pipeline)

### Import (`/import`)

Multi-step flow: Input → Preview → Importing

- **Input step**: Collapsible settings panel (API key + model selector), monospace textarea for note pasting
- **Preview step**: Editable roll info form + shots table with compact inline inputs, wrapped in `FadeIn`
  - Unmatched AI guesses shown as amber warnings with "Add X" links
- **Importing step**: Simple loading message, redirects on success

### Quick Entry (`/quick-entry`)

Rapid single-frame logging optimized for active shooting sessions:
- Roll selector (active rolls prioritized, visual divider for other rolls)
- Roll info bar (camera, film stock, ISO, frame progress indicator)
- 4-column entry form (Frame, f/, Speed, Lens) in a raised card
- `⌘+Enter` keyboard shortcut, session counter, success flash animation
- Previous shots list (reverse-chronological, last 10) with fade-in on latest entry

---

## Atmospheric Effects

### Texture & Depth

Refinement comes from material and texture, layered so nothing sits behind text and contrast is preserved. All in `frontend/src/app.css`:

- **Film grain** — SVG noise `body::after` at 5% opacity, `position: fixed`, `pointer-events: none`, `z-index: 9999`. Subliminal analog quality.
- **Vignette** — a radial gradient painted into the root canvas (`html` `background-image`, darker toward the corners) so it sits *behind* all content. Opaque cards/sidebar cover it; the darkening shows only through the bare page-surface gaps and page edges. Because every glyph is light-on-dark, darkening the surface can only raise text contrast, so it runs strong (0.45 black at the corner) — `text-faint` on a darkened corner is ≈5.8:1, *higher* than on the lit center (≈5.6:1). (Earlier it was a `body::before` overlay at `z-index: 9998`, capped at 0.12 because painting over content could pull faint corner text below AA.)
- **Material cards** — raised rounded panels (`.bg-surface-raised.rounded-lg`) get a lit top "catch-light" edge (`border-top-color`) and a faint top-down sheen (`linear-gradient`); plain cards add a soft elevation shadow. A `:not(.shadow-xl):not(.shadow-2xl)` guard preserves dialogs'/menus' own shadows. This is the "silver/chrome" half of black & silver.

### Dialog Animations

Defined as `@keyframes` in `frontend/src/app.css`:
- `dialog-enter`: scale 0.95 → 1.0 + opacity 0 → 1 over 150ms ease-out
- `backdrop-enter`: opacity 0 → 1 over 100ms ease-out

### Page Section Animations

- `fade-in-up`: translateY(8px) → 0 + opacity 0 → 1 over 200ms ease-out
- Applied via `FadeIn` component with staggered `delay` props
- `success-flash`: green highlight flash (600ms) for Quick Entry save feedback

---

## Design Principles

1. **Graphite & silver, not brown.** Surfaces are near-neutral black/graphite (only a whisper of warmth); text is brushed silver. The single amber accent and the status colors carry the color — the field stays near-monochrome so they read, and so contrast holds.
2. **Precise, not soft.** `rounded-lg` (8px), monospaced data, uppercase section headers — the UI of a precision tool.
3. **Atmospheric, not decorative.** Film grain and subtle gradients create mood without adding visual noise.
4. **Typography tells the story.** Serif for brand identity, mono for data, sans for everything else.
5. **The accent is the safelight.** Amber `#e2a45e` is the single warm light source in a dark room.
6. **Consistent entrance.** Every page section uses `FadeIn` with staggered delays for sequential reveal.
7. **Hover at `/40`.** Card hover borders always use `hover:border-accent/40` — never other opacities.

---

## Dependencies

| Package | Version | Purpose |
|---|---|---|
| `lucide-svelte` | ^0.564.0 | Tree-shakeable SVG icon components |

Self-hosted fonts (in `frontend/static/fonts/`, loaded via `@font-face` in `frontend/src/app.css`):
- [DM Sans](https://fonts.google.com/specimen/DM+Sans) — 400, 500, 600
- [IBM Plex Mono](https://fonts.google.com/specimen/IBM+Plex+Mono) — 400, 500
- [Instrument Serif](https://fonts.google.com/specimen/Instrument+Serif) — 400
