/**
 * Local-time rendering for the naive-UTC timestamps the backend stores.
 *
 * `roll_event.occurred_at` (and other SQLite `datetime('now')` columns) is a
 * naive UTC string like `"2026-06-19 22:17:30"` — space-separated, no timezone
 * suffix. `new Date("2026-06-19 22:17:30")` parses that as LOCAL time in most
 * browsers, which is wrong: it would show the UTC wall-clock as if it were the
 * user's. These helpers explicitly treat the string as UTC, then render in the
 * browser's local timezone (kammerz-j7s).
 *
 * Storage stays UTC — only the rendering is localized.
 */

import { toLocalDayString } from './date';

/**
 * Parse a naive-UTC `"YYYY-MM-DD HH:MM:SS"` (or ISO `"...T..."`) string into a
 * `Date`, treating the wall-clock as UTC. Append a `Z` so the engine reads it as
 * UTC rather than local. A bare `"YYYY-MM-DD"` (no time) is treated as UTC midnight.
 */
export function parseUtcNaive(s: string): Date {
	// Normalize the space separator to `T` so it's a valid ISO datetime, then
	// mark it UTC with a trailing `Z`.
	const iso = s.replace(' ', 'T');
	return new Date(/[zZ]|[+-]\d{2}:?\d{2}$/.test(iso) ? iso : iso + 'Z');
}

/**
 * Local calendar day (`YYYY-MM-DD`) of a naive-UTC timestamp. Built from the
 * Date's LOCAL parts so an event recorded just after UTC midnight is filed under
 * the day the user actually experienced it.
 */
export function localDay(occurredAt: string): string {
	return toLocalDayString(parseUtcNaive(occurredAt));
}

/** Local `HH:MM` time of a naive-UTC timestamp. */
export function formatLocalTime(occurredAt: string): string {
	const d = parseUtcNaive(occurredAt);
	const hh = String(d.getHours()).padStart(2, '0');
	const min = String(d.getMinutes()).padStart(2, '0');
	return `${hh}:${min}`;
}

/**
 * Readable day label (e.g. "Friday, June 19") from a `YYYY-MM-DD` local day
 * string. The string is already a LOCAL calendar day (produced by `localDay`),
 * so it's parsed as local midnight — no UTC conversion here.
 */
export function formatLocalDayLabel(day: string): string {
	// `new Date(...)` never throws — it returns an Invalid Date for bad input —
	// so guard explicitly to fall back to the raw string rather than render the
	// literal "Invalid Date".
	const d = new Date(day + 'T00:00:00');
	if (Number.isNaN(d.getTime())) return day;
	return d.toLocaleDateString(undefined, { weekday: 'long', month: 'long', day: 'numeric' });
}
