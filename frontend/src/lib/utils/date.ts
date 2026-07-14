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
 * (dates are optional) or a complete, real `YYYY-MM-DD` date (year 1800–2100);
 * otherwise a user-facing error message.
 *
 * This is the single source of truth for every Save/Confirm gate, and —
 * mirrored — the backend `validate_date_opt`. The catalog has zero partial
 * dates, so partial forms (`YYYY` / `YYYY-MM`) are no longer accepted — date
 * entry is now the browser-native `<input type="date">`, which can only
 * produce a full date or empty.
 */
export function dateFieldError(value: string | null | undefined): string {
	const v = (value ?? '').trim();
	if (!v) return '';

	// YYYY-MM-DD — validate that the day actually exists
	const full = v.match(/^(\d{4})-(\d{2})-(\d{2})$/);
	if (!full) return 'Use YYYY-MM-DD';

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

/**
 * Coerce a raw (possibly AI-parsed) date into a full `YYYY-MM-DD` best-guess plus
 * an optional note describing any imprecision. Per ADR-0011 an approximate date is
 * stored as a concrete best-guess with the imprecision noted — never a partial — so
 * this lets the import review keep a year-only extraction (e.g. `1998`) instead of
 * hard-blocking the whole roll on it.
 *
 *   '' / null / undefined        -> { date: '',           note: null }
 *   valid full YYYY-MM-DD        -> { date: <same>,       note: null }        (passthrough)
 *   year-only  'YYYY'            -> { date: 'YYYY-01-01',  note: 'approx date: YYYY' }
 *   year-month 'YYYY-MM'         -> { date: 'YYYY-MM-01',  note: 'approx date: YYYY-MM' }
 *   anything else (unparseable)  -> { date: <same>,       note: null }        (passthrough)
 *
 * Reuses `dateFieldError` for validity/range so the 1800–2100 bounds and calendar
 * rules stay in one place. A value we can't confidently best-guess (garbage, or a
 * year/year-month whose candidate is out of range) is passed through UNCHANGED — so
 * `dateFieldError` still flags that field red and the import blocker forces the user
 * to fix it, rather than silently dropping the value.
 */
export function coerceApproxDate(raw: string | null | undefined): { date: string; note: string | null } {
	const v = (raw ?? '').trim();
	if (!v) return { date: '', note: null };
	if (!dateFieldError(v)) return { date: v, note: null }; // already a valid full date

	const ym = v.match(/^(\d{4})-(\d{2})$/);
	if (ym) {
		const candidate = `${ym[1]}-${ym[2]}-01`;
		if (!dateFieldError(candidate)) return { date: candidate, note: `approx date: ${v}` };
	}
	const y = v.match(/^(\d{4})$/);
	if (y) {
		const candidate = `${y[1]}-01-01`;
		if (!dateFieldError(candidate)) return { date: candidate, note: `approx date: ${v}` };
	}
	return { date: v, note: null }; // can't best-guess — leave it for dateFieldError to flag
}

/**
 * Append an optional note `fragment` to an existing `base` notes string. A blank
 * fragment is a no-op; a non-blank one is parenthesized when `base` already has
 * content (`"golden hour (approx date: 1998)"`) or stands alone otherwise. Skips
 * the append when `base` already contains the fragment so a note the AI already
 * added (per the import prompt) isn't duplicated by `coerceApproxDate`'s own note.
 */
export function appendNote(base: string, fragment: string | null | undefined): string {
	const frag = (fragment ?? '').trim();
	if (!frag || base.includes(frag)) return base;
	return base ? `${base} (${frag})` : frag;
}
