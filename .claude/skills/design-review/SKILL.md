---
name: design-review
description: Run a design consistency audit across all frontend pages and components
disable-model-invocation: true
---

# Design Review

Perform a comprehensive design consistency audit of the Kammerz frontend.

## What to Check

### 1. Section Headers
Every section header should use the ledger-line pattern:
- `text-xs font-semibold uppercase tracking-wider text-text-faint`
- With rule line: `<div class="flex-1 border-b border-border-subtle">`
- Or with action button: `justify-between` layout
- **Flag**: any use of `text-sm font-semibold text-text-muted` for section headers

### 2. Card Hover States
- All interactive cards must use `hover:border-accent/40 hover:-translate-y-px`
- **Flag**: any `hover:border-accent/30` or other opacity values

### 3. Badge Component Usage
- Roll statuses must use `<Badge status={...} />`
- **Flag**: any inline status pills (raw `bg-accent/15 text-accent` etc.)

### 4. FadeIn Animations
- Every page should wrap content sections in `<FadeIn>` with staggered delays
- **Flag**: pages missing FadeIn imports or wrappers

### 5. Typography Consistency
- Stat numbers: `font-mono text-2xl font-semibold`
- Roll IDs: `font-mono text-sm font-semibold` (or `text-lg` for prominent display)
- Page titles: `font-display text-xl` (via PageHeader)
- **Flag**: `font-display` used for stat numbers, or inconsistent stat card typography

### 6. Color Token Usage
- No raw hex colors — everything through CSS custom properties
- Status colors use `var(--color-status-*)` tokens
- **Flag**: any hardcoded hex values in component files

### 7. Sidebar Navigation
- Core entity routes separated from utility routes by border
- **Flag**: new routes added to wrong group

## How to Audit

1. Read `UI_DESIGN.md` for current design system rules
2. Glob all `src/routes/**/*.svelte` and `src/lib/components/**/*.svelte`
3. Grep for each anti-pattern listed above
4. Report findings organized by severity (broken → inconsistent → minor)
5. Suggest specific fixes with file paths and line numbers
