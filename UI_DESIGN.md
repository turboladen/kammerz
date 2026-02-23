# UI Design System: "The Darkroom Ledger"

Kammerz's visual identity draws from two analog photography metaphors:

1. **The darkroom** — warm amber safelight glow, deep blacks, chemical warmth
2. **The field log** — photographer's notebook, precise frame numbers, monospaced data, ledger-style precision

This is not skeuomorphism. It's tonal warmth, typographic character, and atmospheric density translated into a modern desktop UI.

---

## Color Palette

All colors are defined as CSS custom properties in `src/app.css` via Tailwind CSS 4's `@theme` block. Every component references these tokens — changing the theme transforms the entire app.

### Surfaces (warm dark tones, like darkroom walls)

| Token | Hex | Usage |
|---|---|---|
| `--color-surface` | `#151210` | Page background, input backgrounds |
| `--color-surface-raised` | `#1e1a17` | Cards, sidebar, dialog panels |
| `--color-surface-overlay` | `#272220` | Dropdown menus, hover states, code badges |

### Borders (warm grays)

| Token | Hex | Usage |
|---|---|---|
| `--color-border` | `#3d3530` | Card borders, input borders, dividers |
| `--color-border-subtle` | `#2d2723` | Sidebar border, softer dividers |

### Text (cream/warm white, like aged paper)

| Token | Hex | Usage |
|---|---|---|
| `--color-text` | `#e8e2d9` | Primary text, headings |
| `--color-text-muted` | `#a09488` | Secondary text, labels, descriptions |
| `--color-text-faint` | `#6d6258` | Tertiary text, hints, timestamps |

### Accent (amber/gold, like a darkroom safelight)

| Token | Hex | Usage |
|---|---|---|
| `--color-accent` | `#d4915c` | Primary buttons, active states, brand |
| `--color-accent-hover` | `#e4a876` | Button hover states |
| `--color-accent-muted` | `#a06830` | Subtle accent backgrounds |

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

### Danger

Use `#c45c4a` for destructive actions (warm red, not pure red).

---

## Typography

Fonts are self-hosted in `static/fonts/` and loaded via `@font-face` declarations in `src/app.css`.

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

### Button (`src/lib/components/ui/Button.svelte`)

Four variants, two sizes:

| Variant | Appearance |
|---|---|
| `primary` | Solid amber accent background |
| `secondary` | Bordered, overlay background |
| `ghost` | Text-only, overlay on hover |
| `danger` | Red-tinted background |

Sizes: `md` (default, px-4 py-2) and `sm` (px-2.5 py-1.5).

### Badge (`src/lib/components/ui/Badge.svelte`)

Roll status pills with a small color dot indicator:
- 1.5px solid dot before the label text
- Background at 10% opacity of the status color
- Text in the status color
- `rounded-full` shape
- Always use `<Badge>` for roll statuses — never inline status pills.

### Dialog (`src/lib/components/ui/Dialog.svelte`)

- Backdrop: `bg-black/50` with `backdrop-blur-sm`
- Panel: `rounded-lg`, `shadow-2xl`, `border-border`
- Title: `font-display text-xl` (Instrument Serif)
- Entry animation: 150ms scale from 0.95 + fade
- Backdrop animation: 100ms fade

### ConfirmDialog (`src/lib/components/ui/ConfirmDialog.svelte`)

Same animations and styling as Dialog, smaller max-width (`max-w-sm`).

### Input, Select, Textarea, ComboInput

- Full border style: `border-border bg-surface`
- Focus: `border-accent ring-1 ring-accent/50`
- Labels: `text-xs font-medium text-text-muted`
- Hints: `text-xs text-text-faint`

### EmptyState (`src/lib/components/ui/EmptyState.svelte`)

Centered message with optional CTA button. Used when lists are empty.

### FadeIn (`src/lib/components/ui/FadeIn.svelte`)

Wraps content in staggered `fade-in-up` animation (200ms, ease-out). Use on all page sections for sequential reveal:
- `delay` prop for staggering (typically 50ms increments between sections)
- Every page should wrap its main content sections in `FadeIn` for consistent entrance animations

---

## Layout

### App Shell (`src/routes/+layout.svelte`)

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

### Sidebar (`src/lib/components/layout/Sidebar.svelte`)

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

**Utility navigation** (separated by `border-t border-border-subtle`):

| Route | Label | Icon |
|---|---|---|
| `/search` | Search | `Search` |
| `/stats` | Stats | `BarChart3` |

### PageHeader (`src/lib/components/layout/PageHeader.svelte`)

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

### Film Grain Texture

Defined in `src/app.css` as a `body::after` pseudo-element:
- SVG noise pattern at 3% opacity
- `position: fixed`, `pointer-events: none`, `z-index: 9999`
- Adds subliminal analog quality without interfering with interaction

### Dialog Animations

Defined as `@keyframes` in `src/app.css`:
- `dialog-enter`: scale 0.95 → 1.0 + opacity 0 → 1 over 150ms ease-out
- `backdrop-enter`: opacity 0 → 1 over 100ms ease-out

### Page Section Animations

- `fade-in-up`: translateY(8px) → 0 + opacity 0 → 1 over 200ms ease-out
- Applied via `FadeIn` component with staggered `delay` props
- `success-flash`: green highlight flash (600ms) for Quick Entry save feedback

---

## Design Principles

1. **Warm, not cool.** Every surface, border, and text color has a brown/amber undertone, never blue/purple.
2. **Precise, not soft.** `rounded-lg` (8px), monospaced data, uppercase section headers — the UI of a precision tool.
3. **Atmospheric, not decorative.** Film grain and subtle gradients create mood without adding visual noise.
4. **Typography tells the story.** Serif for brand identity, mono for data, sans for everything else.
5. **The accent is the safelight.** Amber `#d4915c` is the single warm light source in a dark room.
6. **Consistent entrance.** Every page section uses `FadeIn` with staggered delays for sequential reveal.
7. **Hover at `/40`.** Card hover borders always use `hover:border-accent/40` — never other opacities.

---

## Dependencies

| Package | Version | Purpose |
|---|---|---|
| `lucide-svelte` | ^0.564.0 | Tree-shakeable SVG icon components |

Self-hosted fonts (in `static/fonts/`, loaded via `@font-face` in `src/app.css`):
- [DM Sans](https://fonts.google.com/specimen/DM+Sans) — 400, 500, 600
- [IBM Plex Mono](https://fonts.google.com/specimen/IBM+Plex+Mono) — 400, 500
- [Instrument Serif](https://fonts.google.com/specimen/Instrument+Serif) — 400
