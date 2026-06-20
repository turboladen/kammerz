/**
 * Today's date as a local-time `YYYY-MM-DD` string.
 *
 * Deliberately NOT `new Date().toISOString().split('T')[0]` — `toISOString()`
 * is UTC, so near midnight it can return tomorrow's (or yesterday's) date for
 * users behind/ahead of UTC. This builds the string from the local-time parts.
 */
export function todayLocal(): string {
	return toLocalDayString(new Date());
}

/**
 * A `Date`'s local-time calendar day as a `YYYY-MM-DD` string, built from the
 * local parts (never `toISOString()`, which is UTC — see `todayLocal`). Shared
 * by `todayLocal` and the activity journal's `localDay` in `datetime.ts`.
 */
export function toLocalDayString(d: Date): string {
	const mm = String(d.getMonth() + 1).padStart(2, '0');
	const dd = String(d.getDate()).padStart(2, '0');
	return `${d.getFullYear()}-${mm}-${dd}`;
}

/**
 * Validate a date string for persistence. Returns `''` when the value is empty
 * (dates are optional) or a complete, real date in `YYYY`, `YYYY-MM`, or
 * `YYYY-MM-DD` form (year 1800–2100); otherwise a user-facing error message.
 *
 * This is the single source of truth shared by `DateInput` (live error text),
 * every Save/Confirm gate, and — mirrored — the backend `validate_date_opt`.
 * It is deliberately STRICT: incomplete input like `2026-` is an error here,
 * even though `DateInput` stays lenient about it while the user is mid-type.
 */
export function dateFieldError(value: string | null | undefined): string {
	const v = (value ?? '').trim();
	if (!v) return '';

	// YYYY
	const yearOnly = v.match(/^(\d{4})$/);
	if (yearOnly) {
		const y = parseInt(yearOnly[1]);
		if (y < 1800 || y > 2100) return 'Year out of range';
		return '';
	}
	// YYYY-MM
	const yearMonth = v.match(/^(\d{4})-(\d{2})$/);
	if (yearMonth) {
		const y = parseInt(yearMonth[1]);
		const m = parseInt(yearMonth[2]);
		if (y < 1800 || y > 2100) return 'Year out of range';
		if (m < 1 || m > 12) return 'Month must be 01–12';
		return '';
	}
	// YYYY-MM-DD — validate that the day actually exists
	const full = v.match(/^(\d{4})-(\d{2})-(\d{2})$/);
	if (full) {
		const y = parseInt(full[1]);
		const m = parseInt(full[2]);
		const d = parseInt(full[3]);
		if (y < 1800 || y > 2100) return 'Year out of range';
		if (m < 1 || m > 12) return 'Month must be 01–12';
		// Date constructor rollover trick: an out-of-range day lands in the next
		// month, so the round-tripped parts won't match what we asked for.
		const test = new Date(y, m - 1, d);
		if (test.getFullYear() !== y || test.getMonth() !== m - 1 || test.getDate() !== d) {
			return 'Invalid day for this month';
		}
		return '';
	}
	return 'Use YYYY, YYYY-MM, or YYYY-MM-DD';
}
