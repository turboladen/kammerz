# ADR-0003: Free-text `date_fuzzy` field for imprecise dates

- **Status:** Superseded by [ADR-0004](0004-remove-fuzzy-dates.md)
- **Date:** 2026-06-03
- **Superseded-by:** [ADR-0004](0004-remove-fuzzy-dates.md) (2026-06-21)
- **Related:** [ADR-0004](0004-remove-fuzzy-dates.md) (successor); the original lifecycle-date-capture design is in git history

## Context

Much of the catalog is historical: old rolls whose dates are only approximately
known ("loaded around 12/20", "early October 2025"). The concrete date columns
(`date_loaded`, `date_finished`, …) are `YYYY-MM-DD`, which can't express "around
December 2020". We wanted to record these imprecise dates without inventing a false
precision.

## Decision

Add an optional free-text **`date_fuzzy`** column to both `rolls` and `shots`,
holding the user's verbatim imprecise phrasing alongside (or instead of) the
concrete date fields. The UI surfaced a "Fuzzy Date" input, and search matched it.

## Consequences

- **Positive (at the time):** users could capture imprecise historical dates
  verbatim without guessing a concrete value.
- **Negative (what drove the reversal):** a free-text date field is a dead end for
  any date-driven functionality — you can't sort, filter, or compute on it. It
  duplicated intent with the real date columns, and in practice felt wrong once the
  catalog was populated. See [ADR-0004](0004-remove-fuzzy-dates.md) for the
  replacement (concrete best-guess date + approximation captured in notes).

> This decision was reversed. Retained as the record of why the field existed and
> why it was removed — do not edit the body; see ADR-0004 for current behavior.
