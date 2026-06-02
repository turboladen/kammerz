/**
 * Today's date as a local-time `YYYY-MM-DD` string.
 *
 * Deliberately NOT `new Date().toISOString().split('T')[0]` — `toISOString()`
 * is UTC, so near midnight it can return tomorrow's (or yesterday's) date for
 * users behind/ahead of UTC. This builds the string from the local-time parts.
 */
export function todayLocal(): string {
	const d = new Date();
	const mm = String(d.getMonth() + 1).padStart(2, '0');
	const dd = String(d.getDate()).padStart(2, '0');
	return `${d.getFullYear()}-${mm}-${dd}`;
}

/** Accepted year range — kept in lockstep with DateInput's own bounds. */
const MIN_YEAR = 1800;
const MAX_YEAR = 2100;

/**
 * True iff `s` is a complete, calendar-valid `YYYY-MM-DD` string within the
 * app's accepted year range (1800–2100).
 *
 * Format alone (`/^\d{4}-\d{2}-\d{2}$/`) is NOT enough — it accepts impossible
 * dates like `2026-13-45` or `2026-02-30`. The Date round-trip rejects those:
 * an out-of-range month/day rolls over, so the reconstructed parts no longer
 * match the input. The year bound mirrors DateInput's full-date check so a date
 * the field flags as invalid can never be persisted by a caller that consults
 * only this helper.
 */
export function isValidIsoDate(s: string): boolean {
	const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(s);
	if (!m) return false;
	const y = Number(m[1]);
	const mo = Number(m[2]);
	const d = Number(m[3]);
	if (y < MIN_YEAR || y > MAX_YEAR) return false;
	const dt = new Date(y, mo - 1, d);
	return dt.getFullYear() === y && dt.getMonth() === mo - 1 && dt.getDate() === d;
}
