# ADR-0005: Standardize all time display and entry on 24-hour format

- **Status:** Accepted
- **Date:** 2026-07-13
- **Related:** `kammerz-twp` (implementation)

## Context

Time is presented inconsistently across the app. Event timestamps render 24-hour
via `formatLocalTime` (e.g. `15:17`), while native `<input type="time">` widgets
render in the browser/OS locale — 12-hour with AM/PM in a US locale — even though
their stored value is always 24-hour `HH:mm`. Any display site that falls back to a
default `toLocaleTimeString()` also yields 12-hour. The result is a mix of 12- and
24-hour time depending on where you look.

## Decision

**Standardize on 24-hour time everywhere** (maintainer preference; also unambiguous
and locale-independent). Every time _display_ site routes through a single 24-hour
formatter (`formatLocalTime` is the target); no site uses a default
`toLocaleTimeString()`. For _entry_, values are already stored 24-hour; the
locale-bound native picker display is either accepted as-is (value is correct) or
replaced with a custom 24-hour input — that sub-choice is decided during
implementation and recorded here if it lands as a real component.

## Consequences

- **Positive:** one consistent, unambiguous time format across displays and print;
  no AM/PM drift by locale.
- **Cost:** an audit of every time display/entry site (Timeline/activity, shot time
  in quick-entry, rolls/new, rolls/[id], FrameStrip, print). Tracked by
  `kammerz-twp`.
- **Open sub-decision:** whether to keep the native `<input type="time">` (correct
  value, locale-formatted display) or build a 24-hour input. To be settled in
  `kammerz-twp`; amend this ADR if a custom input is adopted.
