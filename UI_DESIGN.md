# UI Design System: "The Darkroom Ledger"

Kamerz's visual identity draws from two analog photography metaphors:

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
- **Instrument Serif** — Brand name ("Kamerz"), page titles, dialog titles. Editorial quality, used sparingly for warmth. Applied via `.font-display` utility class or Tailwind's `font-display`.

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

- **Brand**: "Kamerz" in Instrument Serif (amber), "film log" subtitle in IBM Plex Mono (uppercase, tracked)
- **Background**: Gradient from `surface-raised` to `surface` (top to bottom)
- **Border**: `border-subtle` (softer than regular border)
- **Icons**: Lucide icons at 16px, `strokeWidth={1.75}`
- **Active state**: 2px left border in accent + `bg-accent/8` + text in accent
- **Hover state**: `bg-surface-overlay` + text lightens
- **Quick Entry**: Dashed border, transitions to solid accent on hover

Navigation items:
| Route | Label | Icon |
|---|---|---|
| `/` | Dashboard | `LayoutDashboard` |
| `/rolls` | Rolls | `Film` |
| `/cameras` | Cameras | `Camera` |
| `/lenses` | Lenses | `Aperture` |
| `/film-stocks` | Film Stocks | `Package` |
| `/labs` | Labs | `FlaskConical` |

### PageHeader (`src/lib/components/layout/PageHeader.svelte`)

- Title in `font-display text-xl`
- Description in `text-sm text-text-faint`
- Border: `border-border-subtle`
- Back link: `text-xs text-text-muted` with arrow, accent on hover
- Action buttons slot on the right

---

## Card Patterns

### List Cards (rolls, cameras, lenses, labs)

```
rounded-lg border border-border bg-surface-raised p-4
transition-all duration-150
hover:border-accent/40 hover:-translate-y-px
```

- `rounded-lg` (8px) — precise, tool-like
- Hover: border glows amber, subtle 1px lift
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

---

## Dashboard Layout

1. **"In the Field"** — Active rolls (loaded/shooting) as prominent amber-tinted cards
2. **Quick Stats** — 4-column grid: Total Rolls, Cameras, Currently Shooting, At Lab
3. **Roll Pipeline** — Horizontal status distribution bar with proportional segments + legend
4. **Needs Attention** — Rolls missing camera assignment or waiting at lab, with icon indicators

Empty state: Camera icon in accent circle, "Start your log" in Instrument Serif, explanatory text, CTAs.

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

---

## Design Principles

1. **Warm, not cool.** Every surface, border, and text color has a brown/amber undertone, never blue/purple.
2. **Precise, not soft.** `rounded-lg` (8px), monospaced data, uppercase section headers — the UI of a precision tool.
3. **Atmospheric, not decorative.** Film grain and subtle gradients create mood without adding visual noise.
4. **Typography tells the story.** Serif for brand identity, mono for data, sans for everything else.
5. **The accent is the safelight.** Amber `#d4915c` is the single warm light source in a dark room.

---

## Dependencies

| Package | Version | Purpose |
|---|---|---|
| `lucide-svelte` | ^0.564.0 | Tree-shakeable SVG icon components |

Self-hosted fonts (in `static/fonts/`, loaded via `@font-face` in `src/app.css`):
- [DM Sans](https://fonts.google.com/specimen/DM+Sans) — 400, 500, 600
- [IBM Plex Mono](https://fonts.google.com/specimen/IBM+Plex+Mono) — 400, 500
- [Instrument Serif](https://fonts.google.com/specimen/Instrument+Serif) — 400
