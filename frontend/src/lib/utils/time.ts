// 24-hour time entry parsing/validation for TimeInput.
//
// The app standardizes on 24-hour time (ADR-0005). Native `<input type="time">`
// renders in the browser's locale (12h AM/PM in en-US) regardless of the OS
// 24-hour setting, so time is entered through a custom text field instead.
// Stored/canonical form is zero-padded `HH:MM` — matching the existing data and
// the backend's strict `validate_time` (`src/validate.rs`).

/**
 * Canonicalize a user time entry to 24-hour `HH:MM`.
 * - `''`/whitespace → `''` (blank is allowed; time is optional)
 * - `H:MM`, `HH:MM`, or colon-less `HHMM` in range → zero-padded `HH:MM`
 * - anything else (out of range, 12h `3:00 PM`, junk, ambiguous 3-digit) → `null`
 */
export function parseTime(raw: string): string | null {
	const v = raw.trim();
	if (!v) return '';
	const m = v.match(/^(\d{1,2}):(\d{2})$/) ?? v.match(/^(\d{2})(\d{2})$/);
	if (!m) return null;
	const h = Number(m[1]);
	const min = Number(m[2]);
	if (h > 23 || min > 59) return null;
	return `${String(h).padStart(2, '0')}:${String(min).padStart(2, '0')}`;
}

/**
 * Live validation message for the field: `''` when blank, still-being-typed
 * (a lone hour / partial minutes), or a valid time; otherwise the 24-hour hint.
 */
export function timeFieldError(raw: string): string {
	const v = raw.trim();
	if (!v) return '';
	if (/^\d{1,2}(:\d{0,1})?$/.test(v)) return '';
	return parseTime(v) === null ? 'Use 24-hour HH:MM' : '';
}
