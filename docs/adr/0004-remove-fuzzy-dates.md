# ADR-0004: Remove `date_fuzzy`; use a concrete date + a notes annotation

- **Status:** Accepted
- **Date:** 2026-06-21
- **Supersedes:** [ADR-0003](0003-fuzzy-date-field.md)
- **Related:** epic `kammerz-qdk` (+ `kammerz-erl`, `kammerz-566`, `kammerz-31c`, `kammerz-btv`)

## Context

The free-text `date_fuzzy` field ([ADR-0003](0003-fuzzy-date-field.md)) proved to be
the wrong shape after real-world use. It carried no machine-usable value and blocked
building anything date-driven (sorting, filtering, timelines) on top of clean date
columns. It also duplicated intent with the concrete `date_*` fields.

## Decision

**Remove `date_fuzzy` everywhere.** Where a date is only approximately known, set the
real date field to a concrete **best-guess** date (e.g. "around 12/20" → `2020-12-01`)
and capture the "around"/approximation phrasing in the record's **notes** instead.
Approximate dates are still first-class input via partial-date acceptance (`YYYY` or
`YYYY-MM`) on the concrete columns — a bare year or year-month is a valid date value.

> **Update:** this partial-date-input allowance was later dropped — see
> [ADR-0011](0011-full-dates-only.md) (dates are always full `YYYY-MM-DD`; approximation
> lives in notes). The removal of `date_fuzzy` decided here still stands.

Scope: drop the columns from `rolls` and `shots` (migration), remove the field from
all DTOs, services, routes, validation, and UI; backfill the 65 existing fuzzy rolls
into concrete date + notes _before_ the drop.

## Consequences

- **Positive:** every date is now a real, comparable value; date-driven features
  (Timeline, sorting, the negatives-pickup deadline) build on clean columns. One
  fewer field to reason about across the stack.
- **Positive:** approximation isn't lost — it lives in notes, human-readable.
- **Migration path:** `kammerz-566` backfilled live data; `kammerz-erl` dropped the
  columns + code + UI; the one-time Apple Notes import had already landed and its
  route carries no fuzzy field, so `kammerz-31c` / `kammerz-btv` were resolved by
  events (verified: zero `date_fuzzy` in code, schema, or data). Epic `kammerz-qdk`
  closed.
- **Note:** the schema migrations still mention `date_fuzzy` (they add it, then
  migration `m20260621_000024` drops it) — that's immutable schema history, not
  current behavior. This ADR is the source of truth for how approximate dates work now.
