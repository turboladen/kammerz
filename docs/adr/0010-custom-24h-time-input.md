# ADR-0010: Custom 24-hour time input (drop the native `<input type="time">`)

- **Status:** Accepted
- **Date:** 2026-07-13
- **Related:** [ADR-0005](0005-24-hour-time.md) (resolves its open entry-widget sub-decision), `frontend/src/lib/components/ui/TimeInput.svelte`, `frontend/src/lib/utils/time.ts`, `src/validate.rs` (`validate_time`), bead `kammerz-twp`

## Context

ADR-0005 standardized the app on 24-hour time and left one sub-decision open: how
to handle _entry_. The native `<input type="time">` renders in the browser's locale
(12-hour AM/PM in en-US) regardless of the OS 24-hour setting — typing `15` shows as
`03 : -- PM`. The stored value and every display are already 24-hour; only the entry
widget wasn't, and it can't be forced to 24h via markup.

## Decision

Enter shot time through a custom **`TimeInput.svelte`**: a plain 24-hour `HH:MM`
text field with inline validation and **no native picker overlay** (overlaying the
native picker would just reintroduce the 12-hour widget). Parsing lives in
`frontend/src/lib/utils/time.ts` — `parseTime` canonicalizes `H:MM` / `HH:MM` /
`HHMM` to zero-padded `HH:MM`; `timeFieldError` drives the live message. The field
normalizes on blur so the stored value matches the backend's strict `validate_time`
(`HH:MM`).

This is deliberately simpler than date entry — time has no partial format and no
locale picker worth keeping, so it's a pure text field.

## Consequences

- **Positive:** time entry is 24-hour on every device/browser, independent of
  locale — the one surface that wasn't enforced-24h now is.
- **Negative:** loses the native time picker's affordances (notably the mobile time
  wheel); the user types the time.
- The backend already validated `HH:MM` (`validate_time`), so no server change was
  needed — the custom input just makes the entry widget agree with the stored form.
