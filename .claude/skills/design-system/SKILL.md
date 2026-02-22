---
name: design-system
description: Automatically applied when building or modifying frontend components, pages, or styles. Ensures all UI work follows the Kammerz "Darkroom Ledger" design system.
---

# Kammerz Design System

When creating or modifying any frontend code (components, pages, styles), always follow the design system documented in `UI_DESIGN.md`. Read it before making changes.

## Quick Reference

### Colors
All colors are CSS custom properties in `src/app.css` via `@theme`. Never use raw hex values.
- Surfaces: `surface`, `surface-raised`, `surface-overlay` (warm dark tones)
- Text: `text` (cream), `text-muted` (warm gray), `text-faint` (dark warm gray)
- Accent: `accent` (#d4915c amber) — the "safelight"
- Danger: `#c45c4a` (warm red, not pure red)

### Typography
- **DM Sans** (`font-sans`) — UI text, labels, buttons
- **IBM Plex Mono** (`font-mono`) — Roll IDs, serial numbers, ISO, aperture, stats numbers
- **Instrument Serif** (`font-display`) — Brand name, page titles, dialog titles only

### Mandatory Patterns
1. **Section headers** — Always `text-xs font-semibold uppercase tracking-wider text-text-faint` with either a rule line or `justify-between` for action buttons. Never `text-sm font-semibold text-text-muted`.
2. **Card hover** — Always `hover:border-accent/40 hover:-translate-y-px`. Never use other accent opacities.
3. **Roll status** — Always use `<Badge status={...} />` component. Never inline status pills.
4. **Status metadata** — Import from `src/lib/utils/status.ts`. Never define inline status maps. Use `getStatusColor()` for typed `RollStatus` values, `getStatusColorSafe()` for untyped strings.
5. **Page animations** — Wrap content sections in `<FadeIn>` with staggered `delay` props (50ms increments).
6. **Error banners** — `rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400`
7. **Empty states** — Use `<EmptyState>` component with Lucide icon snippet.

### Sidebar Navigation
- Core entity nav (Dashboard, Rolls, Cameras, Lenses, Film Stocks, Labs) separated from utility nav (Search, Stats) by `border-t border-border-subtle`.
- New entity routes go in core nav. New utility/analytics routes go in utility nav.
