# ADR-0011: Dates are always full `YYYY-MM-DD` (drop partial-date entry)

- **Status:** Accepted
- **Date:** 2026-07-13
- **Refines:** [ADR-0004](0004-remove-fuzzy-dates.md) — drops its "partial dates are first-class input" allowance; ADR-0004's removal of `date_fuzzy` stands.
- **Related:** `src/validate.rs` (`is_valid_date`), `frontend/src/lib/utils/date.ts` (`dateFieldError`), the `<Input type="date">` call sites (formerly `DateInput`)

## Context

ADR-0004 removed the free-text `date_fuzzy` field but kept partial-date input
(`YYYY` / `YYYY-MM`) as a valid way to record an approximate date on the concrete
columns. The custom `DateInput` component existed almost entirely to support that
partial entry — the native `<input type="date">` can't produce a bare year/month.

In practice partial dates went unused. ADR-0004's _other_ (and preferred) path —
enter a concrete best-guess date, put the "around" phrasing in notes — is what the
300+ roll import actually did, and an audit found **zero** partial dates across the
whole catalog (every `date_*` value is a full `YYYY-MM-DD`). So the partial-entry
machinery (a custom component + partial-tolerant validation on both tiers) was
carrying weight for a capability nobody used.

## Decision

Dates are **always full `YYYY-MM-DD`**. An approximate date is entered as a concrete
best-guess with the imprecision noted — ADR-0004's preferred path, now the only one.

- **Frontend:** delete `DateInput`; enter dates via the browser-native
  `<input type="date">` (through the shared `Input` component) — full dates only,
  and a better picker (esp. mobile). `dateFieldError` tightened to full-or-empty;
  `DateConfirm`'s "enter just a year" copy dropped.
- **Backend:** `validate_date_opt` / `is_valid_date` tightened to reject partials
  (`{field}: use YYYY-MM-DD`).

Note the asymmetry with [ADR-0010](0010-custom-24h-time-input.md): **date** entry
uses the native widget (dates have no locale 12/24h ambiguity), while **time** entry
is custom (native time is locale-12h). Different native-widget defects, different
calls.

## Consequences

- **Positive:** one less custom component; a consistent full-date contract end to
  end — the UI can't enter a partial, and the API won't accept one. Native date
  picker improves mobile entry.
- **Negative:** a truly year-only historical date can no longer be stored as `YYYY`;
  you enter a best-guess full date and note the imprecision (already the catalog's
  actual practice).
- **Negative (accepted):** the native picker doesn't self-render an inline field
  error the way the old `DateInput` did, so the one reachable invalid case — typing
  an out-of-range year (< 1800 / > 2100) — disables Save via the existing
  `dateFieldError` gate but without a per-field message. Rare (the picker enforces a
  valid calendar date otherwise); the Save-disable still prevents bad data.
- **Defense in depth retained:** the stats query keeps its guard against a partial
  `date_loaded` sneaking in via legacy/direct writes (the `date()`-garbage
  regression, `kammerz-4jn`) — a test now seeds one directly to keep that covered
  even though the API rejects partials. The negatives-deadline SQL keeps its
  `length >= 10` guard for the same reason.
